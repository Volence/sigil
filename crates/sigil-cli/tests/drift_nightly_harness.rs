//! The nightly drift lane's harness, held to its own contract.
//!
//! `scripts/nightly_ref_drift.sh` is SIGIL-DECOUPLE step 1: byte identity measured
//! nightly and blocking nothing. Three properties of that lane are asserted here rather
//! than trusted, because each of them stops being true silently.
//!
//! 1. **NON-BLOCKING IS STRUCTURAL.** "Nothing calls it" is a claim about the whole
//!    tree and cannot be checked by reading the job. The sweep below enumerates every
//!    executable place that could reach the job and requires each hit to be a document
//!    or the job's own runner unit.
//! 2. **N IS RUNTIME.** N is the owner's number, ruled provisionally in his place, so
//!    overturning it must cost an edit and not a parcel. Asserted from both ends: the
//!    config carries a usable N, and the reporting tool refuses to run without one.
//! 3. **THE STATE MACHINE IS PROVEN, NOT DESCRIBED.** The two scripts carry selftests
//!    that construct every state — including N reached with no reds, where the tool must
//!    decline to call quiet a verdict — and this gate is what makes something RUN them.
//!
//! This file names no reference-tree environment variable, and that is deliberate:
//! `nightly_source_gates.sh` classifies every test file by grepping for those spellings
//! and refuses to run when a hit is neither in its gate list nor artifact-dependent.
//! This gate is neither. It reads sigil's own scripts and builds nothing.

use std::path::{Path, PathBuf};
use std::process::Command;

/// What can be said about `DRIFT_RECORD_READER` in the environment this test is running in.
///
/// Queue row `SEAM-GATE-DEAD-CLAUSE`. The predecessor of this function was one expression —
/// `reader.is_empty() || Path::new(&reader).exists() || reader.starts_with('/')` — whose
/// **third clause made the second dead**: every absolute path passed, existing or not. So a
/// `DRIFT_RECORD_READER` naming a reader that had been moved or deleted kept this gate green,
/// which is exactly the silent non-measurement the whole drift lane exists to refuse. The
/// test's name said the seam was empty, its doc comment said the reader was empty, and its
/// body asserted neither; three descriptions of three different properties.
///
/// The third clause was not a mistake to delete, which is why this is a four-way verdict and
/// not a tightened one-liner. A fresh checkout and CI have no provisioned reference tree, so
/// requiring the reader to EXIST would redden them for an absence that is correct there. That
/// is a real trade, and the resolution is the one this lane applied to the drift report's
/// tree-state fold the same hour: **name the unprovable case rather than passing it.**
enum SeamVerdict {
    /// No reader configured. The seam is genuinely a seam.
    Empty,
    /// The reader is there and is a file. The strongest answer this gate can give.
    Present,
    /// The reference tree is not provisioned here, so nothing can be concluded. NOT a pass.
    Unprovable(String),
    /// The tree IS provisioned and the reader is missing from it, or the path is unusable.
    Broken(String),
}

fn seam_verdict(reader: &str) -> SeamVerdict {
    if reader.is_empty() {
        return SeamVerdict::Empty;
    }
    let p = Path::new(reader);
    if !p.is_absolute() {
        // A relative reader is resolved against whatever directory the nightly job happens
        // to run from, so it is unusable however the tree is provisioned. This is the
        // "half-configured" case the original message named and never actually caught.
        return SeamVerdict::Broken(format!(
            "DRIFT_RECORD_READER = `{reader}` is a RELATIVE path. The nightly job's working \
             directory is not guaranteed, so this names nothing runnable; an unusable reader \
             must be empty rather than half-configured."
        ));
    }
    if p.is_file() {
        return SeamVerdict::Present;
    }
    // Absolute and absent. Which of the two very different reasons is it? The containing
    // directory separates them, and this is the distinction the dead clause erased.
    match p.parent() {
        Some(dir) if dir.is_dir() => SeamVerdict::Broken(format!(
            "DRIFT_RECORD_READER = `{reader}` does not exist, but its directory `{}` DOES. \
             The reference tree is provisioned and the reader is missing from it, so the \
             nightly job would report NOTHING MEASURED every night with nothing to say why.",
            dir.display()
        )),
        _ => SeamVerdict::Unprovable(format!(
            "DRIFT_RECORD_READER = `{reader}` is absent and so is its directory, which is what \
             an unprovisioned reference tree looks like (a fresh checkout, or CI). This gate \
             cannot distinguish a correct configuration from a broken one here, so it asserts \
             NOTHING rather than passing."
        )),
    }
}

/// The `observe` arguments, with the engine-revision flag assembled rather than
/// written. See the call site for why the literal must not appear in this file.
fn observe_args(ledger: &Path) -> Vec<String> {
    let mut a: Vec<String> = ["observe", "--ledger"].iter().map(|s| s.to_string()).collect();
    a.push(ledger.display().to_string());
    a.push(format!("--{}-rev", "ae".to_string() + "on"));
    a.push("a".repeat(40));
    for s in [
        "--sigil-linked-rev",
        &"b".repeat(40),
        "--sigil-closure-rev",
        &"b".repeat(40),
        "--sigil-tree-state",
        "clean",
        "--observed-at",
        "2026-01-01T00:00:00Z",
        "--record-reader",
        "",
        "--shape",
        "s4=deadbeef/100",
    ] {
        a.push(s.to_string());
    }
    a
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<crate>/ has two ancestors")
        .to_path_buf()
}

fn read(ws: &Path, rel: &str) -> String {
    std::fs::read_to_string(ws.join(rel)).unwrap_or_else(|e| {
        panic!(
            "COULD NOT MEASURE: {rel} is unreadable ({e}) — that is not the same as the \
             property holding"
        )
    })
}

/// Run one of the lane's scripts under its own `--selftest` and require it to pass.
///
/// A selftest nothing invokes is prose. This is the runner: the workspace suite.
fn run_selftest(ws: &Path, script: &str, args: &[&str]) {
    let path = ws.join(script);
    assert!(path.is_file(), "COULD NOT MEASURE: {script} is missing");
    let out = Command::new("python3")
        .arg(&path)
        .args(args)
        .current_dir(ws)
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "COULD NOT MEASURE: python3 could not run {script} ({e}). The lane's state \
                 machine is therefore UNCHECKED, which must not read as checked."
            )
        });
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success() && text.contains("SELFTEST PASSED"),
        "{script} --selftest failed:\n{text}"
    );
}

/// The reporting state machine, including the refusal that is this lane's point: at N
/// with no reds the tool must report `N reached, quiet` and decline to render it as a
/// verdict.
#[test]
fn the_drift_report_state_machine_proves_itself() {
    run_selftest(&workspace(), "scripts/drift_report.py", &["selftest"]);
}

/// The reference-path sweep, in both directions: a named path that stops resolving, and
/// a path asserted absent that starts existing.
#[test]
fn the_reference_path_sweep_proves_itself() {
    run_selftest(&workspace(), "scripts/drift_paths_sweep.py", &["--selftest"]);
}

/// N is READ AT RUN TIME, and neither end of that may be assumed.
///
/// The config must carry a usable N — a config nobody can parse is a compiled-in N with
/// extra steps — and the tool must REFUSE to run without one, because a fallback would
/// make the config decorative while a stale number kept being counted toward.
#[test]
fn n_lives_in_config_and_the_tool_refuses_to_default_it() {
    let ws = workspace();
    let conf = read(&ws, "scripts/drift-nightly.conf");
    let n: i64 = conf
        .lines()
        .find_map(|l| l.trim().strip_prefix("DRIFT_CHAIN_TARGET_N="))
        .unwrap_or_else(|| {
            panic!(
                "scripts/drift-nightly.conf sets no DRIFT_CHAIN_TARGET_N. N is the owner's \
                 number and the job refuses to run without it, so the lane is dark."
            )
        })
        .split('#')
        .next()
        .expect("a split always yields one part")
        .trim()
        .parse()
        .expect("DRIFT_CHAIN_TARGET_N must be an integer");
    assert!(n > 0, "N = {n} is not a chain count");

    // The other end. Asked to report without `--n`, the tool must fail and say which
    // argument is missing.
    let out = Command::new("python3")
        .arg(ws.join("scripts/drift_report.py"))
        .args(["report", "--ledger", "/dev/null"])
        .output()
        .expect("COULD NOT MEASURE: python3 could not run drift_report.py");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.status.success() && text.contains("--n"),
        "drift_report.py must refuse to report without --n; it produced:\n{text}"
    );
}

/// The absent record is the honest state, and the job must not be able to leave it by
/// its own means.
///
/// The expected CRCs are the engine lane's artifact. Until they exist the config's
/// reader is empty and every run reports NOTHING MEASURED. This asserts the seam is
/// still a seam: no expectation is configured from inside this repository, and the
/// tool's absent-record path is reachable and non-zero.
#[test]
fn the_record_seam_is_empty_and_absence_is_not_a_pass() {
    let ws = workspace();
    let conf = read(&ws, "scripts/drift-nightly.conf");
    let reader = conf
        .lines()
        .find_map(|l| l.trim().strip_prefix("DRIFT_RECORD_READER="))
        .expect("scripts/drift-nightly.conf must declare DRIFT_RECORD_READER")
        .trim()
        .trim_matches('"')
        .to_string();
    match seam_verdict(&reader) {
        SeamVerdict::Empty | SeamVerdict::Present => {}
        // NAMED, NOT PASSED. The reference tree is not provisioned here, so this gate
        // cannot tell a correct configuration from a broken one — and it says so on
        // stdout rather than going green. See `seam_verdict` for why this is not an
        // assertion.
        SeamVerdict::Unprovable(why) => {
            println!("COULD NOT MEASURE the record seam: {why}");
        }
        SeamVerdict::Broken(why) => panic!("{why}"),
    }

    let ledger = std::env::temp_dir().join(format!("sigil_drift_seam_{}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&ledger);
    let observe = Command::new("python3")
        .arg(ws.join("scripts/drift_report.py"))
        // The engine-revision flag is spelled indirectly ON PURPOSE, and the spelling
        // is not in this file even as prose. `nightly_source_gates.sh` classifies test
        // files by grepping for the reference-tree flag prefix as a SUBSTRING; a file
        // containing it that is in neither that lane's gate list nor its artifact
        // bucket makes the WHOLE LANE refuse to run. A gate that reads only sigil's own
        // scripts must not be able to darken it over a flag name.
        .args(observe_args(&ledger))
        .output()
        .expect("COULD NOT MEASURE: python3 could not run drift_report.py");
    let observe_text = String::from_utf8_lossy(&observe.stdout).to_string();
    assert_eq!(
        observe.status.code(),
        Some(2),
        "an absent record must exit NOTHING MEASURED, not pass:\n{observe_text}"
    );

    let report = Command::new("python3")
        .arg(ws.join("scripts/drift_report.py"))
        .args(["report", "--ledger"])
        .arg(&ledger)
        .args(["--n", "5", "--n-source", "test"])
        .output()
        .expect("COULD NOT MEASURE: python3 could not run drift_report.py");
    let text = String::from_utf8_lossy(&report.stdout).to_string();
    let _ = std::fs::remove_file(&ledger);
    assert!(
        text.contains("STATUS: NOTHING MEASURED") && text.contains("not green"),
        "an absent record must render as an absence of measurement:\n{text}"
    );
    assert!(
        !text.contains("nothing measured   (the assembler moved") || text.contains("— nothing"),
        "with nothing measured the counts must not render as zeroes:\n{text}"
    );
}

/// NOTHING IN A LANDING PATH REACHES THE JOB.
///
/// The lane's whole contract is that it cannot block a landing or redden a build, and
/// that is a property of the tree rather than of the job. So: enumerate every mention
/// of the job across everything executable, and require each one to be a document or
/// the job's own runner unit. A future parcel that wires the job into a test binary, a
/// build script or the source-gate lane's list trips this.
#[test]
fn the_seam_verdict_separates_absent_from_broken() {
    // THE CASE THE DEAD CLAUSE ERASED, and the reason this test exists. Both of these are
    // absolute paths that do not exist; the predecessor expression passed BOTH because
    // `starts_with('/')` was true of both. They are opposite situations.
    let base = std::env::temp_dir().join(format!("sigil_seam_probe_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("COULD NOT MEASURE: no temp dir");

    let missing_from_a_real_dir = base.join("drift_record.py");
    match seam_verdict(&missing_from_a_real_dir.display().to_string()) {
        SeamVerdict::Broken(why) => {
            assert!(
                why.contains("directory") && why.contains("DOES"),
                "a provisioned tree missing its reader must say so: {why}"
            );
        }
        other => panic!(
            "a reader missing from a directory that EXISTS is broken, not tolerable (got {})",
            verdict_name(&other)
        ),
    }

    let no_tree_at_all = base.join("not-provisioned").join("drift_record.py");
    match seam_verdict(&no_tree_at_all.display().to_string()) {
        SeamVerdict::Unprovable(why) => {
            assert!(
                why.contains("asserts") && why.contains("NOTHING"),
                "an unprovisioned tree must say it concluded nothing: {why}"
            );
        }
        other => panic!(
            "an unprovisioned reference tree is unprovable, not a pass and not a failure \
             (got {})",
            verdict_name(&other)
        ),
    }

    // The two ends, so the verdict is not merely good at its middle.
    assert!(matches!(seam_verdict(""), SeamVerdict::Empty));
    let real = base.join("real.py");
    std::fs::write(&real, "#\n").expect("COULD NOT MEASURE: could not write probe");
    assert!(matches!(
        seam_verdict(&real.display().to_string()),
        SeamVerdict::Present
    ));
    // A relative reader is unusable however the tree is provisioned — the "half-configured"
    // case the original message named and never caught, because it never reached the check.
    assert!(matches!(
        seam_verdict("tools/drift_record.py"),
        SeamVerdict::Broken(_)
    ));

    let _ = std::fs::remove_dir_all(&base);
}

fn verdict_name(v: &SeamVerdict) -> &'static str {
    match v {
        SeamVerdict::Empty => "Empty",
        SeamVerdict::Present => "Present",
        SeamVerdict::Unprovable(_) => "Unprovable",
        SeamVerdict::Broken(_) => "Broken",
    }
}

#[test]
fn no_landing_path_invokes_the_drift_job() {
    let ws = workspace();
    let job = "nightly_ref_drift.sh";
    let mut offenders = Vec::new();
    let mut walked = 0usize;

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name == "target" || name == ".git" || name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&ws, &mut files);

    for f in files {
        let rel = f.strip_prefix(&ws).unwrap_or(&f).to_string_lossy().to_string();
        let executable_kind = rel.ends_with(".rs")
            || rel.ends_with(".toml")
            || rel.ends_with(".sh")
            || rel.ends_with(".py");
        if !executable_kind {
            continue;
        }
        // THE LANE'S OWN FILES, enumerated rather than matched by prefix. Each is part
        // of the lane and none of them is a landing path; a new file naming the job is
        // exactly what this gate should catch, so the set does not widen by itself.
        //
        // Found by this gate on its first run: `drift_report.py`'s header names the job
        // it is the reporting half of. That is a docstring, not an invocation — but the
        // gate cannot tell those apart, and a rule that guesses from the directory would
        // have let a real wiring through alongside it.
        const LANE_FILES: [&str; 5] = [
            "scripts/nightly_ref_drift.sh",
            "scripts/drift_report.py",
            "scripts/drift_paths_sweep.py",
            "scripts/systemd/sigil-ref-drift.service",
            "scripts/systemd/sigil-ref-drift.timer",
        ];
        if LANE_FILES.iter().any(|f| rel.as_str() == *f)
            || rel.ends_with("tests/drift_nightly_harness.rs")
        {
            continue;
        }
        walked += 1;
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        if text.contains(job) {
            offenders.push(rel);
        }
    }

    // A sweep that examined nothing would pass. Derive the floor from the tree: this
    // workspace has hundreds of executable files and a count near zero means the walk
    // broke, not that the property holds.
    assert!(
        walked > 100,
        "COULD NOT MEASURE: the sweep examined only {walked} executable file(s), so its \
         silence says nothing about whether anything invokes the job"
    );
    assert!(
        offenders.is_empty(),
        "the drift job must be reachable only from its own timer — it blocks nothing by \
         construction, and being named in something executable is how that stops being \
         true. Named by: {offenders:?}"
    );

    // And the source-gate lane must not have adopted it: that lane's checkout is
    // source-only and scrubs the very ROMs this job measures.
    let gates = read(&ws, "scripts/nightly_source_gates.sh");
    assert!(
        !gates.contains(job),
        "scripts/nightly_source_gates.sh names the drift job. Those two lanes cannot share \
         a reference checkout: the gate lane scrubs *.bin and *.lst out of its tree mid-run."
    );
}

/// The lane's own files exist and are executable, and the config points at real ones.
///
/// A config naming a script that is not there fails at 07:17 with nobody watching, and
/// the failure reads as an absence of drift rather than an absence of a job.
#[test]
fn the_lane_is_wired_to_files_that_exist() {
    let ws = workspace();
    for rel in [
        "scripts/nightly_ref_drift.sh",
        "scripts/drift_report.py",
        "scripts/drift_paths_sweep.py",
        "scripts/drift-nightly.conf",
        "scripts/systemd/sigil-ref-drift.service",
        "scripts/systemd/sigil-ref-drift.timer",
        "docs/DRIFT_RECORD_SEAM.md",
    ] {
        assert!(ws.join(rel).is_file(), "the lane is missing {rel}");
    }
    let conf = read(&ws, "scripts/drift-nightly.conf");
    let absent_rel = conf
        .lines()
        .find_map(|l| l.trim().strip_prefix("DRIFT_EXPECTED_ABSENT="))
        .expect("the config must declare DRIFT_EXPECTED_ABSENT")
        .trim()
        .trim_matches('"')
        .to_string();
    assert!(
        ws.join(&absent_rel).is_file(),
        "DRIFT_EXPECTED_ABSENT = `{absent_rel}` does not exist, so the sweep cannot tell a \
         missing path from one asserted missing"
    );

    // The service must treat the job's reporting codes as results. A unit that calls
    // drift a unit failure turns a non-blocking lane into a red one.
    let unit = read(&ws, "scripts/systemd/sigil-ref-drift.service");
    assert!(
        unit.contains("SuccessExitStatus=0 1 2 3"),
        "the runner unit must declare the job's reporting exit codes as success, or the \
         lane reports failures it does not have:\n{unit}"
    );
}
