//! `repin` — regenerate `src/pins.rs` from SIGIL'S OWN resolved layout (Stage-3 P4c;
//! the asl-`.lst` parse retired, kill-list row 34).
//!
//! ```text
//! SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   cargo run -p sigil-harness --bin repin -- [--aeon DIR] [--check] [--verbose]
//! ```
//!
//! Resolves the canonical pinned layout NATIVELY for both shapes
//! (`native::sigil_native_symbol_listing` — the fully-resolved symbol table: labels +
//! folded equates incl. `MDDBG__*`, `.emp` locals demangled, section-END markers
//! synthesized), resolves `repin.toml` against both, and rewrites `pins.rs` when any pin
//! moved. `--check` diffs and exits nonzero on drift WITHOUT writing (CI/staleness).
//! `--verbose` also prints every gated region's engine.inc paste block.
//!
//! The pins are DERIVED from sigil's resolve — the placement `native_full_rom` /
//! `native_offcanonical_*` gates prove against the six goldens every build — so a
//! re-pin is a byte-neutral book-keeping refresh, never a new source of truth. The aeon
//! tree comes from `--aeon`, else `AEON_DIR`, else the sibling default; `SIGIL_EMIT`
//! must point at the sound-blob emitter (the resolve builds the sound-on shape).

use std::path::PathBuf;
use std::process::ExitCode;

use sigil_harness::native;
use sigil_harness::repin::{
    diff_pins, load_manifest, render, resolve, strip_provenance, Listing, Provenance,
};

const MANIFEST_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/repin.toml");
const PINS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/pins.rs");

fn fail(msg: &str) -> ExitCode {
    eprintln!("repin: {msg}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut aeon: Option<PathBuf> = None;
    let mut check = false;
    let mut verbose = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--aeon" => match args.next() {
                Some(dir) => aeon = Some(PathBuf::from(dir)),
                None => return fail("--aeon needs a directory argument"),
            },
            "--check" => check = true,
            "--verbose" => verbose = true,
            other => return fail(&format!("unknown argument `{other}` (try --aeon/--check/--verbose)")),
        }
    }
    let aeon = aeon.unwrap_or_else(|| {
        PathBuf::from(
            std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
        )
    });
    if std::env::var("SIGIL_EMIT").map(|v| v.is_empty()).unwrap_or(true) {
        return fail("set SIGIL_EMIT to <sigil>/target/release/emit_sound_blob (the resolve builds sound-on).");
    }

    // The sigil-native symbol source: resolve both shapes, read every symbol's address.
    // Path-independent stamp (the aeon worktree path must not leak into committed pins.rs).
    let (plain, debug) = match (
        native::sigil_native_symbol_listing(&aeon, false),
        native::sigil_native_symbol_listing(&aeon, true),
    ) {
        (Ok((pm, pe)), Ok((dm, de))) => (
            Listing::from_symbols(pm, pe, "plain".into()),
            Listing::from_symbols(dm, de, "debug".into()),
        ),
        (Err(e), _) | (_, Err(e)) => return fail(&e),
    };

    let manifest_src = match std::fs::read_to_string(MANIFEST_PATH) {
        Ok(s) => s,
        Err(e) => return fail(&format!("cannot read {MANIFEST_PATH}: {e}")),
    };
    let manifest = match load_manifest(&manifest_src) {
        Ok(m) => m,
        Err(e) => return fail(&e),
    };
    let resolved = match resolve(&manifest, &plain, &debug) {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    let prov = Provenance {
        plain_path: "sigil-native canonical resolve".into(),
        debug_path: "sigil-native canonical resolve".into(),
        plain_stamp: plain.stamp.clone(),
        debug_stamp: debug.stamp.clone(),
    };
    let generated = render(&resolved, &prov);

    let committed = std::fs::read_to_string(PINS_PATH).unwrap_or_default();
    if strip_provenance(&committed) == strip_provenance(&generated) {
        println!("pins.rs unchanged");
        if verbose {
            println!();
            for block in resolved.gate_blocks() {
                println!("{}", block.render());
            }
        }
        return ExitCode::SUCCESS;
    }

    // ── drift: the D-T10.4 review surface ──
    let changes = diff_pins(&committed, &generated);
    let tests_by_const = resolved.tests_by_const();
    let mut rerun: Vec<&str> = Vec::new();
    let mut changed_consts: Vec<&str> = Vec::new();
    println!("{} pin(s) changed:", changes.len());
    for c in &changes {
        let old = c.old.as_deref().unwrap_or("(new)");
        let new = c.new.as_deref().unwrap_or("(removed)");
        println!("  {}: {old} → {new}{}", c.name, delta_suffix(c.old.as_deref(), c.new.as_deref()));
        changed_consts.push(&c.name);
        if let Some(tests) = tests_by_const.get(&c.name) {
            for t in tests {
                if !rerun.contains(&t.as_str()) {
                    rerun.push(t);
                }
            }
        }
    }
    if !rerun.is_empty() {
        println!();
        println!("rerun hint (affected binaries first, full workspace once at the end):");
        println!("  {}", rerun.join(" "));
    }
    // The engine.inc paste blocks for the gated regions among the changes
    // (every gated region when --verbose).
    let blocks: Vec<String> = resolved
        .gate_blocks()
        .iter()
        .filter(|b| verbose || changed_consts.contains(&b.const_name.as_str()))
        .map(|b| b.render())
        .collect();
    if !blocks.is_empty() {
        println!();
        println!("gated-region resume orgs (reference; the AS else-arms they were pasted into retired with engine.inc / main.asm in K4 inc-6B):");
        println!();
        for b in blocks {
            println!("{b}");
        }
    }

    if check {
        eprintln!("--check: pins.rs is STALE (run `cargo run -p sigil-harness --bin repin`)");
        return ExitCode::FAILURE;
    }
    if let Err(e) = std::fs::write(PINS_PATH, &generated) {
        return fail(&format!("cannot write {PINS_PATH}: {e}"));
    }
    println!();
    println!("wrote {PINS_PATH}");
    ExitCode::SUCCESS
}

/// ` (Δ …)` for single-value numeric pins where a delta is meaningful; empty
/// for added/removed pins and multi-field initializers whose field counts
/// differ.
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
