//! `repin` — regenerate `src/pins.rs` from SIGIL'S OWN resolved layout (Stage-3 P4c;
//! the asl-`.lst` parse retired, kill-list row 34).
//!
//! ```text
//! SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   cargo run -p sigil-harness --bin repin -- \
//!     [--harness-root DIR] [--aeon DIR] [--check] [--verbose]
//! ```
//!
//! WHICH TREE IT WRITES `src/pins.rs` INTO is never taken from link time. `--harness-root`
//! names it, and `refreeze` passes it on every invocation so parent and child cannot
//! resolve different checkouts — the failure that shape prevents is a freeze whose blobs
//! land in one tree and whose pins land in another, reported as a success. Run by hand
//! with no root, it derives the tree it was INVOKED in exactly as `refreeze` does. Either
//! way the tree must carry both markers in
//! [`sigil_harness::harness_root::ROOT_MARKERS`] or the run is refused by name, and
//! [`sigil_harness::harness_root::ROOT_OVERRIDE`] names another tree explicitly. There is
//! no fallback to the tree this binary was compiled in: every run says which tree that
//! was beside the tree it is operating on, and says so in words when they differ.
//!
//! Resolves the canonical pinned layout NATIVELY for both shapes
//! (`native::sigil_native_symbol_listing` — the fully-resolved symbol table: labels +
//! folded equates incl. `MDDBG__*`, `.emp` locals demangled, section-END markers
//! synthesized), resolves `repin.toml` against both, and rewrites `pins.rs` when any pin
//! moved. `--check` diffs and exits nonzero on drift WITHOUT writing (CI/staleness).
//!
//! The pins are DERIVED from sigil's resolve — the placement `native_full_rom` /
//! `native_offcanonical_*` gates prove against the six goldens every build — so a
//! re-pin is a byte-neutral book-keeping refresh, never a new source of truth. The aeon
//! tree comes from `--aeon`, else `AEON_DIR`, else the sibling default; `SIGIL_EMIT`
//! must point at the sound-blob emitter (the resolve builds the sound-on shape).

use std::path::PathBuf;
use std::process::ExitCode;

use sigil_harness::harness_root::{
    announce_root, resolve_passed_root, ROOT_FLAG, ROOT_OVERRIDE,
};
use sigil_harness::native;
use sigil_harness::repin::{
    diff_pins, load_manifest, render, resolve, strip_provenance, Listing, Provenance,
};

fn fail(msg: &str) -> ExitCode {
    eprintln!("repin: {msg}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut aeon: Option<PathBuf> = None;
    let mut harness_root: Option<std::ffi::OsString> = None;
    let mut check = false;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--aeon" => match args.next() {
                Some(dir) => aeon = Some(PathBuf::from(dir)),
                None => return fail("--aeon needs a directory argument"),
            },
            root_flag if root_flag == ROOT_FLAG => match args.next() {
                Some(dir) => harness_root = Some(dir),
                None => return fail(&format!("{ROOT_FLAG} needs a directory argument")),
            },
            "--check" => check = true,
            other => {
                return fail(&format!(
                    "unknown argument `{other}` (try {ROOT_FLAG}/--aeon/--check)"
                ))
            }
        }
    }

    // WHICH TREE, first and unconditionally: everything below reads or writes files under
    // it, and a wrong answer here is silent in every one of them.
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return fail(&format!("cannot read the working directory ({e})")),
    };
    let root = match resolve_passed_root(
        harness_root.as_deref(),
        &cwd,
        std::env::var_os(ROOT_OVERRIDE).as_deref(),
    ) {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    announce_root("repin", &root);
    let manifest_path = root.join("repin.toml");
    let pins_path = root.join("src/pins.rs");
    let aeon = aeon.unwrap_or_else(|| {
        sigil_harness::test_support::aeon_dir()
    });
    if std::env::var("SIGIL_EMIT").map(|v| v.is_empty()).unwrap_or(true) {
        return fail("set SIGIL_EMIT to <sigil>/target/release/emit_sound_blob (the resolve builds sound-on).");
    }

    // The sigil-native symbol source: resolve both shapes, read every symbol's address.
    // Path-independent stamp (the aeon worktree path must not leak into committed pins.rs).
    // Phase-bank label LMAs (T4): a `phase_bank` region resolves its base to the
    // section's LMA (the placement address), not the phase VMA the symbol listing
    // carries. Empty unless the layout holds a `vma:`-windowed bank (soundbankhead).
    // Section extents (REPIN-END): the `section:<name>` boundary spelling measures a
    // region against a section's OWN end (`lma + image_len`), never the successor's base.
    let (plain, debug) = match (
        native::sigil_native_symbol_listing(&aeon, false),
        native::sigil_native_symbol_listing(&aeon, true),
        native::phase_bank_lmas(&aeon, false),
        native::phase_bank_lmas(&aeon, true),
        native::section_extents(&aeon, false),
        native::section_extents(&aeon, true),
        native::section_label_owners(&aeon, false),
        native::section_label_owners(&aeon, true),
    ) {
        (Ok((pm, pe)), Ok((dm, de)), Ok(pl), Ok(dl), Ok(ps), Ok(ds), Ok(po), Ok(do_)) => (
            Listing::from_symbols(pm, pe, "plain".into())
                .with_phase_lma(pl)
                .with_sections(ps)
                .with_label_owners(po),
            Listing::from_symbols(dm, de, "debug".into())
                .with_phase_lma(dl)
                .with_sections(ds)
                .with_label_owners(do_),
        ),
        (Err(e), ..)
        | (_, Err(e), ..)
        | (_, _, Err(e), ..)
        | (_, _, _, Err(e), ..)
        | (_, _, _, _, Err(e), ..)
        | (_, _, _, _, _, Err(e), ..)
        | (.., Err(e), _)
        | (.., Err(e)) => return fail(&e),
    };

    let manifest_src = match std::fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(e) => return fail(&format!("cannot read {}: {e}", manifest_path.display())),
    };
    let manifest = match load_manifest(&manifest_src) {
        Ok(m) => m,
        Err(e) => return fail(&e),
    };
    let resolved = match resolve(&manifest, &plain, &debug) {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    for w in &resolved.warnings {
        eprintln!("repin: warning: {w}");
    }
    let prov = Provenance {
        plain_path: "sigil-native canonical resolve".into(),
        debug_path: "sigil-native canonical resolve".into(),
        plain_stamp: plain.stamp.clone(),
        debug_stamp: debug.stamp.clone(),
    };
    let generated = render(&resolved, &prov);

    let committed = std::fs::read_to_string(&pins_path).unwrap_or_default();
    if strip_provenance(&committed) == strip_provenance(&generated) {
        println!("pins.rs unchanged");
        return ExitCode::SUCCESS;
    }

    // ── drift: the D-T10.4 review surface ──
    let changes = diff_pins(&committed, &generated);
    // THE RERUN HINT IS DERIVED FROM WHAT ACTUALLY REFERENCES EACH CONSTANT, not from
    // `repin.toml`'s `tests` lists. Those lists gate nothing — this hint was their only
    // reader — so an incomplete one could never fail, and measured 2026-09-02, **176 of the
    // 386 symbol rows carrying a pin constant omitted at least one test binary that
    // references it**. The most-omitted were `load_art_port` (41 rows), `game_loop_port`
    // (33) and `repin_pins` (31): a pin moves, the hint names three binaries, and the
    // fourth one that reads it is not mentioned. Hand-correcting 176 rows would have
    // produced a population whose failure mode is "wrong because nobody maintained it",
    // which is the shape this repo rejects elsewhere; deriving deletes the population.
    let rerun = derive_rerun(&root, &changes);
    println!("{} pin(s) changed:", changes.len());
    for c in &changes {
        let old = c.old.as_deref().unwrap_or("(new)");
        let new = c.new.as_deref().unwrap_or("(removed)");
        println!("  {}: {old} → {new}{}", c.name, delta_suffix(c.old.as_deref(), c.new.as_deref()));
    }
    if !rerun.is_empty() {
        println!();
        println!("rerun hint (affected binaries first, full workspace once at the end):");
        println!("  {}", rerun.join(" "));
    }

    if check {
        eprintln!("--check: pins.rs is STALE (run `cargo run -p sigil-harness --bin repin`)");
        return ExitCode::FAILURE;
    }
    if let Err(e) = std::fs::write(&pins_path, &generated) {
        return fail(&format!("cannot write {}: {e}", pins_path.display()));
    }
    println!();
    println!("wrote {}", pins_path.display());
    ExitCode::SUCCESS
}

/// ` (Δ …)` for single-value numeric pins where a delta is meaningful; empty
/// for added/removed pins and multi-field initializers whose field counts
/// differ.
/// Which test binaries actually reference the constants whose pins moved.
///
/// Derived by reading every `crates/*/tests/*.rs` and asking which of them contain the
/// constant's own name. That is a coarse instrument on purpose: it OVER-reports (a
/// mention in a comment counts) and never under-reports, which is the safe direction for
/// a hint whose job is "do not forget to run this". A declared list had the opposite
/// error mode.
///
/// It returns binary names, which is what `cargo test --test <name>` takes.
fn derive_rerun(root: &std::path::Path, changes: &[sigil_harness::repin::PinChange]) -> Vec<String> {
    let mut files: Vec<(String, String)> = Vec::new();
    let crates_dir = root.join("..");
    if let Ok(entries) = std::fs::read_dir(&crates_dir) {
        for e in entries.flatten() {
            let tests = e.path().join("tests");
            if !tests.is_dir() {
                continue;
            }
            if let Ok(rs) = std::fs::read_dir(&tests) {
                for f in rs.flatten() {
                    let p = f.path();
                    if p.extension().and_then(|x| x.to_str()) != Some("rs") {
                        continue;
                    }
                    let Some(stem) = p.file_stem().and_then(|x| x.to_str()) else { continue };
                    if let Ok(text) = std::fs::read_to_string(&p) {
                        files.push((stem.to_string(), text));
                    }
                }
            }
        }
    }
    // AN EMPTY SWEEP IS REPORTED, NEVER PRINTED AS AN EMPTY HINT. The walk assumes `root`
    // is `crates/sigil-harness`, so a `--harness-root` elsewhere finds no test files —
    // and an empty derived hint is indistinguishable from "no test reads these pins",
    // which would read as reassurance. This workspace has hundreds of test files; a count
    // of zero means the walk missed them.
    if files.is_empty() {
        eprintln!(
            "repin: warning: no `crates/*/tests/*.rs` found under {} — the rerun hint below \
             is DERIVED from those files, so it is not a claim that nothing reads these pins",
            crates_dir.display()
        );
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();
    for c in changes {
        for (stem, text) in &files {
            if text.contains(&c.name) && !out.contains(stem) {
                out.push(stem.clone());
            }
        }
    }
    out.sort();
    out
}

fn delta_suffix(old: Option<&str>, new: Option<&str>) -> String {
    let (Some(old), Some(new)) = (old, new) else { return String::new() };
    let nums = |s: &str| -> Vec<i64> {
        let mut out = Vec::new();
        for tok in s.split(|c: char| !c.is_ascii_alphanumeric()) {
            if let Some(hex) = tok.strip_prefix("0x").or_else(|| tok.strip_prefix("0X")) {
                if let Ok(v) = i64::from_str_radix(hex, 16) {
                    out.push(v);
                }
            }
        }
        out
    };
    let (o, n) = (nums(old), nums(new));
    if o.is_empty() || o.len() != n.len() {
        return String::new();
    }
    let deltas: Vec<String> = o
        .iter()
        .zip(&n)
        .filter(|(a, b)| a != b)
        .map(|(a, b)| {
            let d = b - a;
            if d >= 0 { format!("+{d:#X}") } else { format!("-{:#X}", -d) }
        })
        .collect();
    if deltas.is_empty() { String::new() } else { format!(" (Δ {})", deltas.join(", ")) }
}
