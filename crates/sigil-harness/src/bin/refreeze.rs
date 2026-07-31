//! `refreeze` — the ONE-STEP post-flip golden re-freeze (§17 optimization arc).
//!
//! ```text
//! # validate the chain vs the committed blobs (no aeon, no build) — the gate:
//! cargo run -p sigil-harness --bin refreeze -- --check
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
//! This is the SINGLE place a golden moves: repin (pins) + capture (blobs) + derive
//! (sizes) + the provenance chain append, in one command. The hand-edited CRC surface
//! is gone — native_full_rom / native_offcanonical_rom read their expectations FROM
//! provenance.toml.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use sigil_harness::provenance::{self, Target};

const HARNESS_ROOT: &str = env!("CARGO_MANIFEST_DIR");

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
    ]
}

fn fail(msg: impl AsRef<str>) -> ExitCode {
    eprintln!("refreeze: {}", msg.as_ref());
    ExitCode::from(2)
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

fn run_repin(root: &Path) -> Result<(), String> {
    // `cargo run` inherits the environment (CARGO_TARGET_DIR / SIGIL_EMIT / AEON_DIR) and
    // is instant when the binary is already built — robust across the worktree/shared-
    // target split (a hardcoded release-binary path is not). Override via REPIN_BIN.
    let status = if let Ok(bin) = std::env::var("REPIN_BIN") {
        eprintln!("refreeze: running {bin}");
        Command::new(bin).status()
    } else {
        eprintln!("refreeze: running `cargo run --bin repin`");
        Command::new("cargo")
            .args(["run", "-q", "-p", "sigil-harness", "--bin", "repin"])
            .current_dir(root)
            .status()
    }
    .map_err(|e| format!("spawn repin: {e}"))?;
    if !status.success() {
        return Err(format!("repin failed ({status})"));
    }
    Ok(())
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

fn do_freeze(root: &Path, name: &str, ab: &str, note: &str) -> ExitCode {
    let golden = root.join("golden");

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

    let block = provenance::render_entry(name, ab, note, &fresh);
    let mut new_src = src;
    if !new_src.ends_with('\n') {
        new_src.push('\n');
    }
    new_src.push_str(&block);
    if let Err(e) = std::fs::write(golden.join("provenance.toml"), &new_src) {
        return fail(format!("append entry: {e}"));
    }

    // Re-validate the appended chain against the fresh blobs.
    let chain2 = match provenance::parse(&new_src) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let errs = provenance::check(&golden, &chain2);
    if !errs.is_empty() {
        eprintln!("refreeze: appended entry FAILS validation ({}):", errs.len());
        for e in &errs {
            eprintln!("  {e}");
        }
        return ExitCode::from(2);
    }
    println!("refreeze: appended entry `{name}` (ab=\"{ab}\"); chain len {}.", chain2.entry.len());
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let root = PathBuf::from(HARNESS_ROOT);
    let mut args = std::env::args().skip(1);
    let mut check = false;
    let mut freeze_name: Option<String> = None;
    let mut ab = String::new();
    let mut note = String::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--check" => check = true,
            "--freeze" => match args.next() {
                Some(n) => freeze_name = Some(n),
                None => return fail("--freeze needs a parcel name"),
            },
            "--ab" => ab = args.next().unwrap_or_default(),
            "--note" => note = args.next().unwrap_or_default(),
            other => return fail(format!("unknown argument `{other}` (try --check / --freeze NAME --ab REF [--note N])")),
        }
    }
    match (check, freeze_name) {
        (true, None) => do_check(&root),
        (false, Some(name)) => do_freeze(&root, &name, &ab, &note),
        (true, Some(_)) => fail("--check and --freeze are mutually exclusive"),
        (false, None) => fail("nothing to do: pass --check or --freeze NAME --ab REF"),
    }
}
