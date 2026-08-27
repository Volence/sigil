//! `refreeze` — the ONE-STEP post-flip golden re-freeze (§17 optimization arc).
//!
//! ```text
//! # validate the chain vs the committed blobs (no aeon, no build) — the gate:
//! cargo run -p sigil-harness --bin refreeze -- --check
//!
//! # RECORD that the strict full suite ran on the tree carrying the chain tip. Run this
//! # AFTER the freeze is committed; it runs the suite itself, with SIGIL_STRICT_GATE=1
//! # set BY THE TOOL:
//! AEON_DIR=/path/to/aeon cargo run -p sigil-harness --bin refreeze -- \
//!   --attest [--expect-test <a-test-this-parcel-added>]
//!
//! # re-freeze after a byte-CHANGING optimization parcel (regenerates everything):
//! SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//! SIGIL_BUILD=<sigil>/target/release/sigil AEON_DIR=/path/to/aeon \
//!   cargo run -p sigil-harness --bin refreeze -- \
//!     --freeze <parcel-name> --ab <A/B-evidence-ref> [--note "<one line>"]
//! ```
//!
//! `--freeze` runs, IN ORDER: (1) `golden/capture_goldens.sh --write` (rebuild all six
//! ROMs fresh, re-freeze the blobs, restore canonical s4.bin/s4.debug.bin), (2)
//! `golden/derive_offcanonical_sizes.sh` (re-derive the off-canonical size tables from
//! sigil's own layout), (3) `repin` (regenerate `src/pins.rs`). Then it recomputes each
//! target's CRC set from the FRESH blobs (anchor_end read from the freshly-regenerated
//! pins.rs / size-table headers — so a size-changing optimization re-anchors correctly)
//! and either appends a new `[[entry]]` to `provenance.toml` or, when nothing moved,
//! reports the FIXPOINT and appends nothing. So a no-op re-freeze leaves the tree
//! git-clean — the machinery's own regression test.
//!
//! `--freeze` REFUSES unless it can name the aeon revision honestly: `AEON_DIR` must be
//! set, be a git repository, resolve `HEAD`, and be CLEAN. That revision is recorded in
//! the appended entry's `aeon_rev`. See [`resolve_aeon_rev`] for why. `--check` takes no
//! aeon tree and is deliberately unaffected.
//!
//! WHICH TREE IT WRITES INTO is derived from where it is INVOKED, not from where it was
//! built: the repository toplevel of the working directory, which for a linked worktree
//! is that worktree. The tree must carry both markers in
//! [`sigil_harness::harness_root::ROOT_MARKERS`] or the run is refused by name;
//! [`sigil_harness::harness_root::ROOT_OVERRIDE`] names another tree explicitly and is
//! verified the same way. Every run prints the tree it was built from beside the tree it
//! is operating on, so a binary older than the question being asked of it is visible
//! rather than silently authoritative. Every child tool is TOLD that root — see
//! [`run_repin`] — so the child cannot resolve a different one.
//!
//! This is the SINGLE place a golden moves: repin (pins) + capture (blobs) + derive
//! (sizes) + the provenance chain append, in one command. The hand-edited CRC surface
//! is gone — native_full_rom / native_offcanonical_rom read their expectations FROM
//! provenance.toml.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use sigil_harness::harness_root::{
    announce_root, repin_invocation, resolve_harness_root, ROOT_OVERRIDE,
};
use sigil_harness::provenance::{self, AppendGate, StrictRun, Superseded, Target};
use sigil_harness::strict_census;

/// target-key -> (committed golden blob, off-canonical size-table file or "" for the
/// canonical shapes whose EndOfRom lives in pins.rs).
fn target_sources() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("s4", "s4.bin", ""),
        ("s4_debug", "s4.debug.bin", ""),
        ("demo", "demo.bin", "demo.txt"),
        ("demo_debug", "demo.debug.bin", "demo_debug.txt"),
        ("config_a", "config_a.bin", "config_a.txt"),
        ("config_b", "config_b.bin", "config_b.txt"),
        // The 7th target: the crash-report-OFF (lean) shape — no MD Debugger island,
        // no deb2 appendix, faults route at ReleaseFault.
        ("lean", "lean.bin", "lean.txt"),
    ]
}

/// Refusing a `--supersede-tip` that has nothing to abandon. Checked BOTH before the
/// rebuild (so the usage error is free) and again at the append (so the decision is made
/// against the chain as it actually is when the entry is written).
const SUPERSEDE_WITHOUT_A_RED_RUN: &str =
    "--supersede-tip was passed, but the tip records no strict run at all. An entry can only \
     be ABANDONED once `--attest` has recorded that its suite came back RED; otherwise \
     abandoning is just a way to skip the run, and the ratchet dissolves into a formality.";

const SUPERSEDE_OF_A_GREEN_TIP: &str =
    "--supersede-tip was passed, but the tip's strict run PASSED. A green entry is not being \
     abandoned; drop the flag.";

fn fail(msg: impl AsRef<str>) -> ExitCode {
    eprintln!("refreeze: {}", msg.as_ref());
    ExitCode::from(2)
}

/// Resolve the aeon revision `--freeze` is about to freeze FROM, or refuse.
///
/// Until this existed the freeze path never looked at `AEON_DIR` at all — it passed the
/// environment through to `capture_goldens.sh`, whose `${AEON_DIR:-/home/volence/…/aeon}`
/// fallback silently builds against the owner's LIVE working tree, which routinely carries
/// hours of uncommitted content edits. The existing `--ab`/anchor guard fires on byte
/// MOVEMENT, so it is blind to this by construction: the wrong tree gets frozen and the
/// record says nothing about it.
///
/// This is not new policy. `docs/OVERSEER.md`'s landing lane already requires freezing from
/// a clean checkout of a committed SHA; the rule was simply unenforceable. Every refusal
/// names the variable, the path, and what specifically was wrong.
///
/// `--freeze` ONLY. `--check` is documented as "no aeon, no build" and must keep working
/// with `AEON_DIR` unset — it reads committed blobs and toml and nothing else.
fn resolve_aeon_rev() -> Result<String, String> {
    let dir = std::env::var("AEON_DIR").map_err(|_| {
        "AEON_DIR is not set. --freeze builds the goldens from the aeon tree, so it must \
         record WHICH tree; unset, capture_goldens.sh would silently fall back to the \
         owner's live checkout. Set AEON_DIR to a clean checkout of a committed aeon SHA."
            .to_string()
    })?;
    let path = PathBuf::from(&dir);
    if !path.is_dir() {
        return Err(format!("AEON_DIR={dir} is not a directory."));
    }

    let git = |args: &[&str]| -> Result<String, String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(&path)
            .args(args)
            .output()
            .map_err(|e| format!("spawn git {args:?} in AEON_DIR={dir}: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git {} failed in AEON_DIR={dir}: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };

    git(&["rev-parse", "--git-dir"]).map_err(|e| {
        format!("AEON_DIR={dir} is not a git repository, so its revision cannot be named ({e}).")
    })?;
    let rev = git(&["rev-parse", "HEAD"]).map_err(|e| {
        format!("AEON_DIR={dir}: cannot resolve HEAD, so the freeze cannot name a revision ({e}).")
    })?;
    if !provenance::is_full_sha(&rev) {
        return Err(format!("AEON_DIR={dir}: HEAD resolved to `{rev}`, not a 40-char SHA."));
    }

    // Dirty means the bytes about to be frozen were built from something that is not any
    // committed revision, so `aeon_rev` would be a LIE rather than merely absent. Checked
    // BEFORE the build, since the build itself writes into this tree.
    let dirty = git(&["status", "--porcelain"])?;
    if !dirty.is_empty() {
        let lines: Vec<&str> = dirty.lines().collect();
        let shown: Vec<&str> = lines.iter().take(10).copied().collect();
        return Err(format!(
            "AEON_DIR={dir} is DIRTY at {rev} ({} change(s)); the bytes it builds would not \
             correspond to any committed revision, so aeon_rev could not name them honestly. \
             Freeze from a clean checkout of a committed SHA. Changes:\n  {}{}",
            lines.len(),
            shown.join("\n  "),
            if lines.len() > shown.len() { "\n  …" } else { "" }
        ));
    }
    Ok(rev)
}

/// Parse `EndOfRom` for the two canonical shapes from the freshly-written pins.rs (NOT
/// the compile-time constant — a fresh repin may have moved it).
fn parse_pins_end(pins_src: &str, name: &str) -> Result<usize, String> {
    for line in pins_src.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix(&format!("pub const {name}: usize =")) {
            let v = rest.trim().trim_end_matches(';').trim();
            let v = v.strip_prefix("0x").unwrap_or(v);
            return usize::from_str_radix(v, 16)
                .or_else(|_| v.parse::<usize>())
                .map_err(|e| format!("pins.rs {name}: parse {v}: {e}"));
        }
    }
    Err(format!("pins.rs: constant {name} not found"))
}

/// Parse `# assembled_end=0x...` from an off-canonical size-table header.
fn parse_size_table_end(src: &str, file: &str) -> Result<usize, String> {
    for line in src.lines() {
        if let Some(rest) = line.trim().strip_prefix("# assembled_end=") {
            let v = rest.trim();
            let v = v.strip_prefix("0x").unwrap_or(v);
            return usize::from_str_radix(v, 16).map_err(|e| format!("{file} assembled_end {v}: {e}"));
        }
    }
    Err(format!("{file}: no `# assembled_end=` header"))
}

/// Read authoritative EndOfRom per target from the fresh pins.rs + size tables.
fn authoritative_anchor_ends(root: &Path) -> Result<BTreeMap<String, usize>, String> {
    let pins_src = std::fs::read_to_string(root.join("src/pins.rs"))
        .map_err(|e| format!("read pins.rs: {e}"))?;
    let mut out = BTreeMap::new();
    for (key, _golden, size_file) in target_sources() {
        let end = match key {
            "s4" => parse_pins_end(&pins_src, "ASSEMBLED_LEN")?,
            "s4_debug" => parse_pins_end(&pins_src, "DEBUG_ASSEMBLED_LEN")?,
            _ => {
                let p = root.join("golden/offcanonical_sizes").join(size_file);
                let s = std::fs::read_to_string(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
                parse_size_table_end(&s, size_file)?
            }
        };
        out.insert(key.to_string(), end);
    }
    Ok(out)
}

fn golden_map() -> BTreeMap<String, String> {
    target_sources().into_iter().map(|(k, g, _)| (k.to_string(), g.to_string())).collect()
}

/// Run one regeneration script, inheriting the environment (SIGIL_EMIT / SIGIL_BUILD /
/// AEON_DIR flow through). A nonzero exit aborts the freeze.
fn run_script(script: &Path, args: &[&str]) -> Result<(), String> {
    eprintln!("refreeze: running {} {}", script.display(), args.join(" "));
    let status = Command::new("bash")
        .arg(script)
        .args(args)
        .status()
        .map_err(|e| format!("spawn {}: {e}", script.display()))?;
    if !status.success() {
        return Err(format!("{} failed ({status})", script.display()));
    }
    Ok(())
}

/// Spawn `repin` against `root`. Both spawning shapes are built by
/// [`sigil_harness::harness_root::repin_invocation`], which is where the handover of the
/// root is documented and gated.
fn run_repin(root: &Path) -> Result<(), String> {
    let inv = repin_invocation(root, std::env::var_os("REPIN_BIN"));
    let rendered: Vec<String> =
        inv.args.iter().map(|a| a.to_string_lossy().into_owned()).collect();
    eprintln!("refreeze: running {} {}", inv.program.to_string_lossy(), rendered.join(" "));
    let status = Command::new(&inv.program)
        .args(&inv.args)
        .current_dir(&inv.cwd)
        .status()
        .map_err(|e| format!("spawn repin: {e}"))?;
    if !status.success() {
        return Err(format!("repin failed ({status})"));
    }
    Ok(())
}

/// The sigil repository root (this crate sits two levels below it).
fn sigil_root(harness_root: &Path) -> Result<PathBuf, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(harness_root)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("spawn git in {}: {e}", harness_root.display()))?;
    if !out.status.success() {
        return Err(format!("{} is not inside a git repository", harness_root.display()));
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

fn git_at(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("spawn git {args:?} in {}: {e}", dir.display()))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed in {}: {}",
            args.join(" "),
            dir.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Resolve the SIGIL revision the strict suite is about to run on, or refuse.
///
/// Clean is required for two independent reasons, and both matter:
///
///   * the record must name a tree that EXISTS. A dirty tree's bytes correspond to no
///     committed revision, so `sigil_rev` would be a lie rather than merely absent —
///     the same argument `resolve_aeon_rev` makes about `AEON_DIR`.
///   * the suite itself contains `version_reports_the_head_of_the_tree_it_was_built_from`,
///     which compares the `sigil` binary's baked-in revision against HEAD at ASSERTION
///     time. Attesting from a committed, unmoving HEAD is what keeps that test and this
///     field describing the same tree; attesting mid-freeze, before the goldens are
///     committed, would name the freeze's PARENT and quietly describe the wrong bytes.
///
/// That is why `--attest` runs AFTER the freeze is committed, not between freeze and
/// commit.
fn resolve_sigil_rev(root: &Path) -> Result<String, String> {
    git_at(root, &["rev-parse", "--git-dir"])
        .map_err(|e| format!("{} is not a git repository ({e}).", root.display()))?;
    let rev = git_at(root, &["rev-parse", "HEAD"])?;
    if !provenance::is_full_sha(&rev) {
        return Err(format!("{}: HEAD resolved to `{rev}`, not a 40-char SHA.", root.display()));
    }
    let dirty = git_at(root, &["status", "--porcelain"])?;
    if !dirty.is_empty() {
        let lines: Vec<&str> = dirty.lines().collect();
        let shown: Vec<&str> = lines.iter().take(10).copied().collect();
        return Err(format!(
            "the sigil tree at {} is DIRTY at {rev} ({} change(s)), so the suite's subject is \
             not any committed revision and `sigil_rev` could not name it honestly. COMMIT \
             THE FREEZE FIRST, then attest: `--attest` deliberately runs after the golden \
             blobs, pins.rs and provenance.toml are committed, which is also what keeps \
             `version_reports_the_head_of_the_tree_it_was_built_from` describing the same \
             tree this record names. Changes:\n  {}{}",
            root.display(),
            lines.len(),
            shown.join("\n  "),
            if lines.len() > shown.len() { "\n  …" } else { "" }
        ));
    }
    Ok(rev)
}

/// The DECLARED test population: what the built binaries say they will run.
///
/// The landing bar has always been "`passed + ignored` equals the declared count", and
/// until now that comparison lived only in `docs/OVERSEER.md` as prose — a human ran a
/// grep and compared by eye. Two overseers believed it enforced; nothing in the harness
/// enforced it. A binary that silently does not run at all takes its whole population
/// out of the totals, and the remaining suites still report `ok`: another smaller green,
/// the same shape as the strict-body floor this parcel replaces.
///
/// `--list` is used rather than a source grep for the same reason the census is derived
/// rather than committed: it enumerates what the RUNNER will schedule, so a test that
/// exists in source but is not collected shows as a difference instead of agreeing by
/// coincidence. It also cannot be edited into agreement.
///
/// LOUD ON UNMEASURABLE: a listing that fails, or enumerates nothing, is an error and
/// never a zero — a zero would compare equal to a run that executed nothing.
fn listed_tests(root: &Path) -> Result<usize, String> {
    let out = Command::new("cargo")
        .args(["test", "--release", "--workspace", "--", "--list"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("spawn `cargo test -- --list`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`cargo test --release --workspace -- --list` exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let n = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.trim_end().ends_with(": test"))
        .count();
    if n == 0 {
        return Err(
            "`cargo test -- --list` enumerated ZERO tests. The listing broke; that is not the \
             same as a workspace with no tests"
                .to_string(),
        );
    }
    Ok(n)
}

/// What one strict suite run produced. Every number here is read out of the run; none is
/// copied from the chain.
struct RunResult {
    suites: usize,
    passed: usize,
    failed: usize,
    ignored: usize,
    skips: usize,
    failing: Vec<String>,
    strict_bodies: usize,
    /// The strict-gate POPULATION the run reached, not just its size. A count can only
    /// be read; a population can be diffed against the census, which is what lets a
    /// refusal name the gate that went dark instead of reporting a smaller number.
    witness: strict_census::Witness,
    exit_ok: bool,
}

/// Parse a finished suite log. `witness` is the strict-body witness file.
fn measure_run(log: &str, witness: &Path) -> RunResult {
    let (mut suites, mut passed, mut failed, mut ignored, mut skips) = (0, 0, 0, 0, 0);
    let mut failing = Vec::new();
    for line in log.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("test result:") {
            // `ok. 12 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; …`
            suites += 1;
            let toks: Vec<&str> = rest.split_whitespace().collect();
            for w in toks.windows(2) {
                let n: usize = match w[0].parse() {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                match w[1].trim_end_matches(';') {
                    "passed" => passed += n,
                    "failed" => failed += n,
                    "ignored" => ignored += n,
                    _ => {}
                }
            }
        } else if t.starts_with("test ") && t.ends_with("... FAILED") {
            let name = t.trim_start_matches("test ").trim_end_matches("... FAILED").trim();
            failing.push(name.to_string());
        }
        // BOTH spellings. The landing bar greps `skip:`, and a live sibling parcel found
        // 27 sites that say `skipping` instead — invisible to that grep while reporting
        // green. A matcher that inherits the same blind spot would under-count while
        // still looking like a witness, which is the worst failure mode a count has.
        if line.contains("skip:") || line.contains("skipping") {
            skips += 1;
        }
    }
    failing.sort();
    failing.dedup();
    // DISTINCT call sites, not total writes: one strict-gated body reached twice is one
    // body. Measured: `section_row_fixture.rs`'s shared `gate_on()` helper fires three
    // times (once per test) and is ONE site. Absent/unreadable reads as an empty
    // population, which `--attest` refuses — never as green.
    let w = strict_census::parse_witness(&std::fs::read_to_string(witness).unwrap_or_default());
    RunResult {
        suites,
        passed,
        failed,
        ignored,
        skips,
        failing,
        strict_bodies: w.sites.len(),
        witness: w,
        exit_ok: false,
    }
}

/// Whether the run's log shows a test of this name having executed.
fn test_ran(log: &str, name: &str) -> bool {
    log.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("test ") && t.contains(name) && t.contains(" ... ")
    })
}

/// THE MONOTONIC RATCHET, as a decision that can be exercised.
///
/// The census is derived from source, so it moves WITH a deletion and cannot see one: an
/// edit that removes a strict-gated test removes its expectation in the same stroke. The
/// only artifact that can witness an absence from the tree behind you is one from the
/// past, and the chain is one — an append-only record of runs that HAPPENED, not a
/// statement of what should happen. Nothing here is hand-maintained to pass.
///
/// That is only worth anything if a shrink FAILS. A chain that records each run's number
/// and compares nothing is a diary: the gate goes dark, a smaller number lands in a newer
/// entry, and nobody reads it. So a shrink returns `Err` and NOTHING is recorded.
///
/// And a failure with no honest exit is the trap this repo already names. `Superseded` is
/// the chain's vocabulary for "this was legitimate", and it does not reach here: it
/// requires the tip to carry a RED strict run, while a retired gate produces a GREEN one.
/// Leaving no exit would make the honest operator's only move an edit to a PRIOR
/// committed entry — the forged field. So the exit is an explicit argument: unreachable
/// by accident, not an edit to anything committed, and it makes the shrink a thing
/// somebody SAID rather than a thing that quietly happened.
///
/// `prev` is the last recorded `strict_bodies` in the chain, or `None` while the rule is
/// unarmed. Returns the line to report, or the refusal.
fn strict_bodies_ratchet(
    prev: Option<usize>,
    now: usize,
    retire: Option<&str>,
) -> Result<String, String> {
    match (prev, retire) {
        // Unarmed: no previous run to compare against. Self-disarming, in the same shape
        // as the two ratchets already in the tree. Reported as `ratchet:`, NEVER as
        // `skip:` — this lane's strict bar requires zero `skip:` lines, and a rule
        // reporting its own dormancy must not spend one.
        (None, None) => Ok(
            "ratchet: no entry in this chain records a strict run yet, so there is no \
             previous population to compare against. It arms permanently with this \
             attestation."
                .to_string(),
        ),
        (None, Some(why)) => Err(format!(
            "--retired-strict-gates was passed ({why:?}) but no entry in this chain records a \
             strict run, so there is no previous count for anything to have fallen from."
        )),
        (Some(p), None) if now < p => Err(format!(
            "strict_bodies FELL from {p} to {now} since the last recorded strict run. The \
             census is green, which means the tree no longer DECLARES the missing gate(s) \
             either — a whole strict-gated test or file was removed, which no source-derived \
             census can see. That may be a deliberate retirement, but it is not something an \
             attestation may record silently. Restore the gate, or say why it is gone: \
             `--retired-strict-gates \"<one line>\"`. Nothing was recorded."
        )),
        (Some(p), Some(why)) if now < p => {
            Ok(format!("strict_bodies fell {p} -> {now}, ACKNOWLEDGED: {why}"))
        }
        (Some(p), Some(why)) => Err(format!(
            "--retired-strict-gates was passed ({why:?}) but strict_bodies did not fall \
             ({p} -> {now}). An acknowledgement with nothing to acknowledge trains the reflex \
             of passing it by default, which retires the ratchet."
        )),
        (Some(p), None) => Ok(format!("strict-body ratchet: {p} -> {now}, held.")),
    }
}

/// `--attest` — RUN the strict full suite and record it against the chain tip.
///
/// The tool runs the suite itself rather than accepting a log or a hand-written field,
/// because a field a human can type is a claim and not a witness. In particular it sets
/// `SIGIL_STRICT_GATE=1` on the child ITSELF: the missing environment variable is the
/// whole defect this closes, and a recipe that asks an operator to remember it is the
/// recipe that already failed twice.
///
/// What the run must MATCH is derived, not floored. [`strict_census`] reads the
/// population of strict-gate consultations out of the test tree before the suite starts,
/// and the run's witness is set-diffed against it afterwards. The old bar —
/// `strict_bodies != 0` — was satisfiable by the failure it existed to catch: a deleted,
/// `#[ignore]`d or unguarded gate lands at 28 of 29 and records a pass, so a gate going
/// dark read back as a smaller green.
fn do_attest(
    harness_root: &Path,
    expect: &[String],
    log_arg: Option<&str>,
    retire: Option<&str>,
) -> ExitCode {
    let golden = harness_root.join("golden");
    let root = match sigil_root(harness_root) {
        Ok(r) => r,
        Err(e) => return fail(e),
    };

    // (0) The blobs in this tree must BE the tip. Attesting a tree whose goldens do not
    // match the entry would record a run about different bytes than the entry names.
    let src = match std::fs::read_to_string(golden.join("provenance.toml")) {
        Ok(s) => s,
        Err(e) => return fail(format!("read provenance.toml: {e}")),
    };
    let chain = match provenance::parse(&src) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let errs = provenance::check(&golden, &chain);
    if !errs.is_empty() {
        eprintln!("refreeze --attest: the chain does not hold, so nothing here can be attested:");
        for e in &errs {
            eprintln!("  {e}");
        }
        return ExitCode::from(2);
    }
    let tip = chain.tip().unwrap();
    let number = chain.entry.len();
    if tip.strict.is_some() {
        return fail(format!(
            "the tip `{}` (entry #{number}) already records a strict run; re-attesting would \
             append a second [entry.strict] table and break the file. An entry is frozen: if \
             its record is wrong, that is a repair, not a re-run.",
            tip.name
        ));
    }

    // (0b) THE EXPECTATION, DERIVED. Before the suite runs, because a census that cannot
    // be taken should cost nothing to find out, and because the tree is pinned clean at
    // (1) — so what is derived here is what the run will measure.
    //
    // This replaces a FLOOR. `strict_bodies == 0` refused only the total absence of the
    // flag; every partial loss — a gate deleted, `#[ignore]`d, filtered, or stripped of
    // its guard — landed at 28 out of 29 and recorded a pass. A gate going dark showed
    // up as a smaller green, which is the one shape a witness must never have.
    let census = match strict_census::census(&root.join("crates")) {
        Ok(c) => c,
        Err(e) => return fail(format!(
            "refusing to attest — the strict-gate census could not be derived, so there is \
             nothing to hold the run's witness to. {e}"
        )),
    };
    eprintln!(
        "refreeze --attest: expecting {} strict-gate site(s) across {} test(s) in the tree",
        census.sites.len(),
        census.tests.len()
    );

    // (1) WHICH TREES. Both resolved and vetted before anything expensive runs.
    let sigil_rev = match resolve_sigil_rev(&root) {
        Ok(r) => r,
        Err(e) => return fail(format!("refusing to attest — {e}")),
    };
    let aeon_rev = match resolve_aeon_rev() {
        Ok(r) => r,
        Err(e) => return fail(format!("refusing to attest — {e}")),
    };
    let aeon_dir = std::env::var("AEON_DIR").unwrap_or_default();
    if let Some(want) = tip.aeon_rev.as_deref() {
        if want != aeon_rev {
            return fail(format!(
                "AEON_DIR is at aeon {aeon_rev}, but the tip `{}` (entry #{number}) was frozen \
                 from aeon {want}. A suite run against a different revision than these goldens \
                 came from is not a test of this entry. Point AEON_DIR at a clean checkout of \
                 {want}.",
                tip.name
            ));
        }
    }

    // (2) The golden identities, READ FROM THE BLOBS — never copied out of the entry.
    let mut goldens = BTreeMap::new();
    for (key, t) in &tip.targets {
        match provenance::recompute_target(&golden, &t.golden, t.anchor_end) {
            Ok((crc, size, _)) => {
                goldens.insert(key.clone(), format!("{crc}/{size}"));
            }
            Err(e) => return fail(format!("cannot read golden `{key}`: {e}")),
        }
    }

    // (3) RUN IT.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("target"))
        .join("attest");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return fail(format!("create {}: {e}", dir.display()));
    }
    let witness = dir.join(format!("witness-{stamp}.txt"));
    let log_path = log_arg.map(PathBuf::from).unwrap_or_else(|| dir.join(format!("suite-{stamp}.log")));
    let _ = std::fs::remove_file(&witness);

    // The log is STAMPED before cargo writes a byte into it. A suite log does not name
    // the tree it measured, and a landing run from the wrong worktree reads green AND
    // better than the bar; the stamp is what makes the log answerable after the fact.
    let branch = git_at(&root, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|_| "?".into());
    let header = format!(
        "# sigil strict attestation run\n\
         # pwd            {}\n\
         # sigil HEAD     {sigil_rev}\n\
         # sigil branch   {branch}\n\
         # AEON_DIR       {aeon_dir}\n\
         # aeon HEAD      {aeon_rev}\n\
         # provenance tip {} (entry #{number})\n\
         # witness        {}\n\
         # command        SIGIL_STRICT_GATE=1 cargo test --release --workspace --no-fail-fast -- --nocapture\n\n",
        root.display(),
        tip.name,
        witness.display()
    );
    let mut file = match std::fs::File::create(&log_path) {
        Ok(f) => f,
        Err(e) => return fail(format!("create {}: {e}", log_path.display())),
    };
    {
        use std::io::Write;
        if let Err(e) = file.write_all(header.as_bytes()) {
            return fail(format!("stamp {}: {e}", log_path.display()));
        }
    }
    eprintln!("refreeze --attest: running the strict full suite; log -> {}", log_path.display());
    eprintln!("refreeze --attest: (tail it: tail -f {})", log_path.display());
    let err_half = match file.try_clone() {
        Ok(f) => f,
        Err(e) => return fail(format!("clone log handle: {e}")),
    };
    let status = Command::new("cargo")
        .args(["test", "--release", "--workspace", "--no-fail-fast", "--", "--nocapture"])
        .current_dir(&root)
        // SET BY THE TOOL, never asked of the operator. This one missing variable is the
        // entire defect being closed.
        .env("SIGIL_STRICT_GATE", "1")
        .env(sigil_harness::test_support::STRICT_WITNESS_VAR, &witness)
        .stdout(std::process::Stdio::from(file))
        .stderr(std::process::Stdio::from(err_half))
        .status();
    let status = match status {
        Ok(s) => s,
        Err(e) => return fail(format!("spawn cargo test: {e}")),
    };

    let log = std::fs::read_to_string(&log_path).unwrap_or_default();
    let mut run = measure_run(&log, &witness);
    run.exit_ok = status.success();

    // (4) REFUSALS. Every one of these is a run that cannot be classified, and an
    // unclassifiable run is never green.
    if run.suites == 0 {
        return fail(format!(
            "the run produced NO `test result:` line, so nothing about it can be measured \
             (cargo exit {status}). See {}.",
            log_path.display()
        ));
    }
    if run.passed == 0 {
        return fail(format!(
            "the run executed {} test binaries but 0 tests passed — that is a run that could \
             not be measured, not a green one. See {}.",
            run.suites,
            log_path.display()
        ));
    }
    // THE VACUITY REFUSAL — the TOTAL-loss case, kept for its diagnosis rather than for
    // its coverage. The census comparison below subsumes it (an empty witness is missing
    // every declared site), but only this branch can say WHY a run reached nothing: the
    // flag never took effect. Zero on its own was never the bar; a gate going dark
    // lands at 28 of 29 and clears it, which is the defect the census closes.
    if run.strict_bodies == 0 {
        return fail(format!(
            "the run reached ZERO strict-gated bodies, so `SIGIL_STRICT_GATE=1` did not take \
             effect in the test processes — every `strict_gate()`-guarded port and co-link \
             gate early-returned and the green means nothing. This is precisely the state \
             chains 169 and 170 landed in. Witness file: {} (expected one `file:line` per \
             strict-gated body reached). Nothing was recorded.",
            witness.display()
        ));
    }
    let missing: Vec<&String> = expect.iter().filter(|n| !test_ran(&log, n)).collect();
    if !missing.is_empty() {
        return fail(format!(
            "these --expect-test name(s) did not execute in the run: {missing:?}. A green log \
             that does not contain the landed code's own test is a green log about other code. \
             See {}.",
            log_path.display()
        ));
    }
    let head_now = git_at(&root, &["rev-parse", "HEAD"]).unwrap_or_default();
    if head_now != sigil_rev {
        return fail(format!(
            "HEAD moved during the run ({sigil_rev} -> {head_now}); the suite measured a tree \
             this record could not name. Re-run on a settled checkout."
        ));
    }
    // THE POPULATION COMPARISON, and the reason `strict_bodies == 0` is no longer the
    // bar. The census above says WHICH strict-gate consultations this tree declares and
    // WHICH tests carry them; the witness says which the run actually reached. A set
    // difference names the gate that went dark. A count could only have said "28".
    //
    // Applied on the GREEN path only, deliberately. A red run's coverage is not what an
    // attestation is about, and its record is the only thing that unlocks a supersede —
    // refusing to write it would deadlock the chain on exactly the failure the record
    // exists to capture. Under strict, a missing-reference path panics, so a red run is
    // also the one place the census legitimately disagrees with the witness.
    if run.failed == 0 && run.exit_ok {
        // THE DECLARED-COUNT RECONCILIATION, moved out of prose. `passed + failed +
        // ignored` must account for every test the runner says it will schedule; a
        // binary that silently did not run leaves the rest reporting `ok`.
        match listed_tests(&root) {
            Err(e) => return fail(format!(
                "the run cannot be reconciled against the declared test population, so its \
                 totals describe an unknown fraction of the suite. {e}"
            )),
            Ok(listed) => {
                let ran = run.passed + run.failed + run.ignored;
                if ran != listed {
                    return fail(format!(
                        "the run accounted for {ran} test(s) ({} passed + {} failed + {} \
                         ignored) but the binaries declare {listed}. {} test(s) were never \
                         reported by any `test result:` line — a binary that did not run takes \
                         its whole population out of the totals while every other suite still \
                         says `ok`. Nothing was recorded. Log: {}",
                        run.passed,
                        run.failed,
                        run.ignored,
                        listed.saturating_sub(ran),
                        log_path.display()
                    ));
                }
                eprintln!(
                    "refreeze --attest: declared-count reconciliation: {ran} accounted for, \
                     {listed} declared."
                );
            }
        }

        let defects = strict_census::defects(&census, &run.witness);
        if !defects.is_empty() {
            eprintln!("refreeze --attest: {}", strict_census::summary(&census, &run.witness));
            return fail(format!(
                "the run's strict-gate population does not match the one this tree declares \
                 ({} difference(s)). The suite was GREEN, which is precisely the state a gate \
                 going dark produces: a smaller green. Nothing was recorded.\n  {}\nWitness: \
                 {}\nLog: {}",
                defects.len(),
                defects.join("\n  "),
                witness.display(),
                log_path.display()
            ));
        }
        eprintln!("refreeze --attest: {}", strict_census::summary(&census, &run.witness));

        // THE MONOTONIC RATCHET, secondary and self-arming. The census cannot see a
        // whole strict-gated test being DELETED — that edit removes the expectation and
        // the gate together — so the only witness to it is a MEMORY of the previous
        // population. The chain has one: the last `[entry.strict]` it recorded.
        // Measured 2026-08-27: the live chain records ZERO strict runs, so this rule is
        // not yet in force; it arms itself at the first attestation, in the same
        // self-disarming shape as the two ratchets already in the tree. Reported as
        // `ratchet:` and NEVER as `skip:` — this lane's strict bar requires zero `skip:`
        // lines, and a rule reporting its own dormancy must not spend one.
        match strict_bodies_ratchet(
            chain.entry.iter().rev().find_map(|e| e.strict.as_ref()).map(|p| p.strict_bodies),
            run.strict_bodies,
            retire,
        ) {
            Ok(note) => eprintln!("refreeze --attest: {note}"),
            Err(e) => return fail(e),
        }
    }

    let outcome = if run.failed > 0 {
        provenance::OUTCOME_FAILED
    } else if !run.exit_ok {
        return fail(format!(
            "cargo exited {status} but no test reported a failure across {} binaries — the run \
             could not be classified as pass or fail (a build error, a harness abort, or a \
             linker failure). See {}.",
            run.suites,
            log_path.display()
        ));
    } else {
        provenance::OUTCOME_PASSED
    };

    // A `skip:` under strict is a gate that reported green while measuring nothing. It is
    // RECORDED loudly rather than refused: closing the last of those holes is another
    // lane's live work, and a gate that co-opts a peer's open bar wedges them on day one.
    // The number rides in the chain permanently, so a reader can see what the green was
    // worth.
    if run.skips > 0 {
        eprintln!(
            "refreeze --attest: WARNING — {} `skip:` line(s) in a STRICT run. Each is a gate \
             that reported green while measuring nothing. Recording the count in the chain; \
             this lane's bar is zero.",
            run.skips
        );
        for l in log.lines().filter(|l| l.contains("skip:")).take(20) {
            eprintln!("    {}", l.trim());
        }
    }

    let record = StrictRun {
        outcome: outcome.to_string(),
        sigil_rev,
        aeon_rev,
        strict_bodies: run.strict_bodies,
        suites: run.suites,
        passed: run.passed,
        failed: run.failed,
        ignored: run.ignored,
        skips: run.skips,
        ran_at: format!("unix:{stamp}"),
        failing: run.failing.clone(),
        expected_tests: expect.to_vec(),
        goldens,
    };

    let block = provenance::render_strict(&record);
    let (_, chain2) = match provenance::append_block(&golden.join("provenance.toml"), &src, &block) {
        Ok(v) => v,
        Err(e) => return fail(format!("append [entry.strict]: {e}")),
    };
    let errs = provenance::check(&golden, &chain2);
    if !errs.is_empty() {
        eprintln!("refreeze --attest: the appended record FAILS validation ({}):", errs.len());
        for e in &errs {
            eprintln!("  {e}");
        }
        return ExitCode::from(2);
    }
    println!(
        "refreeze --attest: recorded {outcome} on tip `{}` (entry #{number}) — {} strict bodies, \
         {} suites, {} passed / {} failed / {} ignored, {} skip: line(s).",
        chain2.tip().unwrap().name,
        record.strict_bodies,
        record.suites,
        record.passed,
        record.failed,
        record.ignored,
        record.skips
    );
    println!("          log: {}", log_path.display());
    if outcome == provenance::OUTCOME_FAILED {
        println!(
            "          The run was RED ({} test(s)). This entry can no longer be attested; the \
             next `--freeze` may pass `--supersede-tip \"<why>\"` to record it as abandoned.",
            record.failing.len()
        );
        for n in record.failing.iter().take(30) {
            println!("            {n}");
        }
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn do_check(root: &Path) -> ExitCode {
    let golden = root.join("golden");
    let src = match std::fs::read_to_string(golden.join("provenance.toml")) {
        Ok(s) => s,
        Err(e) => return fail(format!("read provenance.toml: {e}")),
    };
    let chain = match provenance::parse(&src) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let errs = provenance::check(&golden, &chain);
    if errs.is_empty() {
        println!("refreeze --check: OK (tip `{}`, chain len {})", chain.tip().unwrap().name, chain.entry.len());
        ExitCode::SUCCESS
    } else {
        eprintln!("refreeze --check: {} violation(s):", errs.len());
        for e in &errs {
            eprintln!("  {e}");
        }
        ExitCode::from(2)
    }
}

fn do_freeze(root: &Path, name: &str, ab: &str, note: &str, supersede: Option<&str>) -> ExitCode {
    let golden = root.join("golden");

    // (-2) THE PROSE, CHECKED BEFORE ANYTHING. `--ab`, `--note` and `--supersede-tip` are
    // sentences typed on the command line, and a sentence the ledger cannot show verbatim
    // is a usage error: it should cost no aeon tree and no rebuild, and it should name the
    // character rather than mangle it. Checked AGAIN at the append, over the full entry,
    // because the derived fields are not known until then.
    for (flag, v) in [("--freeze", name), ("--ab", ab), ("--note", note)]
        .into_iter()
        .chain(supersede.map(|r| ("--supersede-tip", r)))
    {
        if let Some(f) = provenance::fault_in_prose(flag, v) {
            return fail(f);
        }
    }

    // (-1) THE GATE, CONSULTED FIRST. Two different jobs, and both belong before any
    // work: a MISUSED `--supersede-tip` is a usage error and must cost nothing — not an
    // aeon tree, not a rebuild — and a freeze that is going to be refused at the append
    // should say so before it spends twenty minutes finding out.
    //
    // It is only a WARNING and not an early refusal, because the rule is about APPENDING:
    // a byte-neutral re-freeze appends nothing, and that no-op is this machinery's own
    // regression test — it must keep working on a tip that has not been attested yet.
    if let Some(c) = std::fs::read_to_string(golden.join("provenance.toml"))
        .ok()
        .and_then(|s| provenance::parse(&s).ok())
    {
        match (provenance::append_gate(&c), supersede) {
            (AppendGate::Ratchet(_), Some(_)) => return fail(SUPERSEDE_WITHOUT_A_RED_RUN),
            (AppendGate::Allowed, Some(_)) => return fail(SUPERSEDE_OF_A_GREEN_TIP),
            (AppendGate::Refused(m), _) | (AppendGate::NeedsSupersede(m), None) => eprintln!(
                "refreeze: WARNING — if this freeze moves bytes it will REFUSE to append: {m}"
            ),
            _ => {}
        }
    }

    // (0) WHICH TREE. Resolved and vetted before anything is built, both because a refusal
    // should cost nothing and because the build writes into this tree — after it, "clean"
    // is no longer a question that can be asked.
    let aeon_rev = match resolve_aeon_rev() {
        Ok(r) => r,
        Err(e) => return fail(format!("refusing to freeze — {e}")),
    };
    eprintln!("refreeze: freezing from aeon {aeon_rev} (clean)");

    // (1) blobs, (2) size tables, (3) pins — the three regen steps.
    if let Err(e) = run_script(&golden.join("capture_goldens.sh"), &["--write"]) {
        return fail(e);
    }
    if let Err(e) = run_script(&golden.join("derive_offcanonical_sizes.sh"), &[]) {
        return fail(e);
    }
    if let Err(e) = run_repin(root) {
        return fail(e);
    }

    // Recompute the tip set from the FRESH blobs + FRESH anchor_ends.
    let ends = match authoritative_anchor_ends(root) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    let gmap = golden_map();
    let fresh: BTreeMap<String, Target> = match provenance::recompute_targets(&golden, &gmap, &ends) {
        Ok(t) => t,
        Err(e) => return fail(e),
    };

    let src = match std::fs::read_to_string(golden.join("provenance.toml")) {
        Ok(s) => s,
        Err(e) => return fail(format!("read provenance.toml: {e}")),
    };
    let chain = match provenance::parse(&src) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };

    if provenance::equals_tip(&chain, &fresh) {
        println!("refreeze: FIXPOINT — the regenerated goldens match tip `{}`; nothing appended.", chain.tip().unwrap().name);
        println!("          (byte-neutral re-freeze; the tree stays git-clean.)");
        return ExitCode::SUCCESS;
    }

    // THE STRICT-ATTESTATION GATE. Checked here — after the FIXPOINT branch above and
    // before the append — because the rule is about building a new entry ON TOP OF an
    // unproven one. A re-freeze that changes nothing proves nothing new and is exempt.
    let mut superseded_block = String::new();
    match provenance::append_gate(&chain) {
        AppendGate::Ratchet(m) => {
            // `ratchet:`, never `skip:`. This lane's strict bar requires zero `skip:`
            // lines and this is not a missing reference; it is a rule that has not armed.
            eprintln!("{m}");
            if supersede.is_some() {
                return fail(SUPERSEDE_WITHOUT_A_RED_RUN);
            }
        }
        AppendGate::Allowed => {
            if supersede.is_some() {
                return fail(SUPERSEDE_OF_A_GREEN_TIP);
            }
        }
        AppendGate::NeedsSupersede(m) => match supersede {
            None => return fail(m),
            Some(reason) if reason.trim().is_empty() => {
                return fail("--supersede-tip needs a one-line reason")
            }
            Some(reason) => {
                if let Some(f) = provenance::fault_in_prose("--supersede-tip", reason) {
                    return fail(f);
                }
                let tip_name = chain.tip().map(|t| t.name.clone()).unwrap_or_default();
                eprintln!(
                    "refreeze: ABANDONING tip `{tip_name}` — its strict run was red; this entry \
                     `{name}` is recorded as its successor."
                );
                superseded_block = provenance::render_superseded(&Superseded {
                    by: name.to_string(),
                    reason: reason.to_string(),
                });
            }
        },
        AppendGate::Refused(m) => return fail(m),
    }

    // Discipline pre-check: an anchor that moved needs a real A/B ref.
    if ab.trim().is_empty() || ab == provenance::ASL_WITNESS {
        if let Ok(tip) = chain.tip() {
            let moved: Vec<&String> = fresh
                .iter()
                .filter(|(k, t)| tip.targets.get(*k).map(|p| p.anchor_crc != t.anchor_crc).unwrap_or(true))
                .map(|(k, _)| k)
                .collect();
            if !moved.is_empty() {
                return fail(format!(
                    "anchor(s) moved {:?} but --ab is empty/sentinel — a byte-changing freeze needs an A/B evidence ref",
                    moved
                ));
            }
        }
    }

    // REFUSE BEFORE THE WRITE. `--ab`, `--note` and `--supersede-tip` are prose typed on
    // the command line; a value this ledger cannot show verbatim is reported by name and
    // position while the author still has the sentence to fix, and while the file on
    // disk is untouched.
    let faults = provenance::entry_faults(name, ab, &aeon_rev, note, &fresh);
    if !faults.is_empty() {
        eprintln!("refreeze: --freeze refused, the entry cannot be written faithfully:");
        for f in &faults {
            eprintln!("  {f}");
        }
        return ExitCode::from(2);
    }

    let block = provenance::render_entry(name, ab, &aeon_rev, note, &fresh);
    // ORDER MATTERS: `[entry.superseded]` attaches to the LAST `[[entry]]` in the file,
    // so it must be written while the OLD tip is still last — then the successor's own
    // `[[entry]]` block follows. One append, no surgery, every predecessor untouched.
    let append = format!("{superseded_block}{block}");
    let (_, chain2) = match provenance::append_block(&golden.join("provenance.toml"), &src, &append)
    {
        Ok(v) => v,
        Err(e) => return fail(format!("append entry: {e}")),
    };
    let errs = provenance::check(&golden, &chain2);
    if !errs.is_empty() {
        eprintln!("refreeze: appended entry FAILS validation ({}):", errs.len());
        for e in &errs {
            eprintln!("  {e}");
        }
        return ExitCode::from(2);
    }
    println!(
        "refreeze: appended entry `{name}` (ab=\"{ab}\", aeon_rev={aeon_rev}); chain len {}.",
        chain2.entry.len()
    );
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return fail(format!("cannot read the working directory ({e})")),
    };
    let root = match resolve_harness_root(&cwd, std::env::var_os(ROOT_OVERRIDE).as_deref()) {
        Ok(r) => r,
        Err(e) => return fail(e),
    };
    announce_root("refreeze", &root);
    let mut args = std::env::args().skip(1);
    let mut check = false;
    let mut attest = false;
    let mut freeze_name: Option<String> = None;
    let mut ab = String::new();
    let mut note = String::new();
    let mut supersede: Option<String> = None;
    let mut expect: Vec<String> = Vec::new();
    let mut log: Option<String> = None;
    let mut retire: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--check" => check = true,
            "--attest" => attest = true,
            "--freeze" => match args.next() {
                Some(n) => freeze_name = Some(n),
                None => return fail("--freeze needs a parcel name"),
            },
            "--ab" => ab = args.next().unwrap_or_default(),
            "--note" => note = args.next().unwrap_or_default(),
            "--supersede-tip" => match args.next() {
                Some(r) => supersede = Some(r),
                None => return fail("--supersede-tip needs a one-line reason"),
            },
            "--expect-test" => match args.next() {
                Some(t) => expect.push(t),
                None => return fail("--expect-test needs a test name"),
            },
            "--log" => log = args.next(),
            "--retired-strict-gates" => match args.next() {
                Some(r) if !r.trim().is_empty() => retire = Some(r),
                _ => return fail(
                    "--retired-strict-gates needs a one-line reason naming what was retired"
                ),
            },
            other => return fail(format!(
                "unknown argument `{other}` (try --check / --attest [--expect-test NAME] \
                 [--retired-strict-gates WHY] / --freeze NAME --ab REF [--note N] \
                 [--supersede-tip WHY])"
            )),
        }
    }
    if attest && (check || freeze_name.is_some()) {
        return fail("--attest is its own mode: it runs the suite and records the result");
    }
    if attest {
        return do_attest(&root, &expect, log.as_deref(), retire.as_deref());
    }
    if !expect.is_empty() {
        return fail("--expect-test applies to --attest, which runs the suite");
    }
    if retire.is_some() {
        return fail("--retired-strict-gates applies to --attest, which measures the population");
    }
    match (check, freeze_name) {
        (true, None) => do_check(&root),
        (false, Some(name)) => do_freeze(&root, &name, &ab, &note, supersede.as_deref()),
        (true, Some(_)) => fail("--check and --freeze are mutually exclusive"),
        (false, None) => fail("nothing to do: pass --check, --attest, or --freeze NAME --ab REF"),
    }
}

#[cfg(test)]
mod child_handover {
    /// There is ONE way this tool spawns `repin`, and it is the one that hands over the
    /// resolved root.
    ///
    /// The gate reads the source rather than the behaviour because what it forbids is a
    /// SECOND spawn appearing later — a shortcut for one shape, an `if` for a special
    /// case — which no run of the existing path would ever exercise. `REPIN_BIN` is
    /// singled out because it skips the rebuild unconditionally and is therefore the
    /// likeliest way a child from another tree gets run.
    #[test]
    fn repin_is_spawned_only_through_the_shared_invocation_builder() {
        let src = include_str!("refreeze.rs");
        let body = {
            let start = src.find("fn run_repin(").expect("nothing to measure: run_repin is gone");
            let rest = &src[start..];
            let end = rest.find("\n}\n").expect("run_repin has no end");
            &rest[..end]
        };
        assert!(
            body.contains("repin_invocation(root, std::env::var_os(\"REPIN_BIN\"))"),
            "the spawn must take its program and arguments from the shared builder, with \
             the prebuilt-binary override flowing through it: {body}"
        );
        assert_eq!(
            body.matches("Command::new").count(),
            1,
            "one spawn, and it is the one built above: {body}"
        );
        assert!(
            body.contains("Command::new(&inv.program)") && body.contains(".args(&inv.args)"),
            "the spawn must use the built program AND the built arguments — using one \
             without the other is how the root gets dropped on a shape: {body}"
        );
        // Split so this gate does not count its own prose. Measured over the CODE only —
        // everything above the first test module.
        let token = concat!("REPIN_", "BIN");
        let code = &src[..src.find("#[cfg(test)]").expect("no test module marker to cut at")];
        assert_eq!(
            code.matches(token).count(),
            1,
            "a second reading of the prebuilt-binary override is a second spawning path \
             in waiting"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A libtest log fragment: two binaries, one failing test, one `skip:` line.
    const LOG: &str = "\
test provenance_chain_holds ... ok
test aeon_dir_matches_the_provenance_tip ... ok
skip: reference not at /nowhere/s4.bin (set AEON_DIR)
note: skipping the oracle cross-check, g++ unavailable
test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s

test boot::header_matches ... FAILED
test result: FAILED. 40 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 2.0s
";

    /// EVERY BRANCH OF THE RATCHET, reached. The live chain records no strict run, so
    /// the shrink path cannot be exercised by a real `--attest` today — and a branch
    /// nothing reaches reads exactly like one that passed, which is this parcel's whole
    /// subject. So it is reached here instead of left as decoration.
    #[test]
    fn the_strict_body_ratchet_fails_on_a_shrink_and_has_a_named_exit() {
        // Unarmed — reported as `ratchet:`, never `skip:`.
        let m = strict_bodies_ratchet(None, 29, None).expect("unarmed must not refuse");
        assert!(m.starts_with("ratchet:"), "{m}");
        assert!(!m.contains("skip:"), "a dormant rule must not spend a skip line: {m}");

        // Held, and grown — both fine.
        assert!(strict_bodies_ratchet(Some(29), 29, None).is_ok());
        assert!(strict_bodies_ratchet(Some(29), 30, None).is_ok());

        // A SHRINK IS A FAILURE, not a diary entry. This is the joint: if this recorded
        // the new number instead, the chain would be a census with extra steps.
        let e = strict_bodies_ratchet(Some(29), 28, None)
            .expect_err("a shrink must refuse, not record the smaller population");
        assert!(e.contains("FELL from 29 to 28"), "{e}");
        assert!(e.contains("--retired-strict-gates"), "the refusal must name its exit: {e}");

        // The named exit works, and says so out loud.
        let m = strict_bodies_ratchet(Some(29), 28, Some("retired the pitchtable co-link gate"))
            .expect("an acknowledged shrink must be allowed");
        assert!(m.contains("ACKNOWLEDGED") && m.contains("pitchtable"), "{m}");

        // And it cannot be worn by default: passing it with nothing to acknowledge is a
        // refusal, because a flag people pass every time is a flag that retires the rule.
        assert!(strict_bodies_ratchet(Some(29), 29, Some("why")).is_err());
        assert!(strict_bodies_ratchet(None, 29, Some("why")).is_err());
    }

    fn witness(lines: &[&str]) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
        f.flush().unwrap();
        f
    }

    #[test]
    fn counts_are_summed_across_every_test_binary() {
        let w = witness(&["a.rs:1"]);
        let r = measure_run(LOG, w.path());
        assert_eq!(r.suites, 2, "one per `test result:` line");
        assert_eq!(r.passed, 42, "2 + 40");
        assert_eq!(r.failed, 1);
        assert_eq!(r.ignored, 4, "1 + 3");
        // BOTH spellings counted: `skip:` and the `skipping` the landing bar's grep
        // cannot see.
        assert_eq!(r.skips, 2, "a `skipping` line is a skip the landing bar's grep misses");
        assert_eq!(r.failing, vec!["boot::header_matches".to_string()]);
    }

    /// The witness counts DISTINCT strict-gated bodies: one body reached twice (a test
    /// binary re-running it, or several tests sharing a helper) is one body.
    #[test]
    fn the_witness_counts_distinct_call_sites() {
        let w = witness(&["a.rs:1", "a.rs:1", "b.rs:9", "", "  b.rs:9  "]);
        assert_eq!(measure_run("", w.path()).strict_bodies, 2);
    }

    /// LOUD ON UNMEASURABLE. An unreadable or absent witness must read as ZERO, which
    /// `--attest` refuses — never as "fine, the file just was not there".
    #[test]
    fn an_unwritten_witness_reads_as_zero_not_as_absent() {
        assert_eq!(measure_run(LOG, Path::new("/nonexistent/witness")).strict_bodies, 0);
    }

    /// A suite that produced no `test result:` line at all is unmeasurable, and the
    /// zeroes here are what make `--attest` refuse rather than record a green.
    #[test]
    fn a_log_with_no_result_line_measures_zero() {
        let w = witness(&["a.rs:1"]);
        let r = measure_run("error: could not compile `sigil-cli`\n", w.path());
        assert_eq!((r.suites, r.passed), (0, 0));
    }

    #[test]
    fn expect_test_sees_a_test_that_executed_and_not_one_that_did_not() {
        assert!(test_ran(LOG, "aeon_dir_matches_the_provenance_tip"));
        assert!(test_ran(LOG, "boot::header_matches"), "a FAILED test still executed");
        assert!(!test_ran(LOG, "a_gate_that_was_filtered_out"));
        // A mere MENTION is not an execution: the name has to appear on a libtest
        // result line, not in some test's own output.
        assert!(!test_ran("running my_new_gate now\n", "my_new_gate"));
    }
}
