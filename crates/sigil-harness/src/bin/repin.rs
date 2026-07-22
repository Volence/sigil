//! `repin` — regenerate `src/pins.rs` from the aeon listings (D-T10.1).
//!
//! ```text
//! cargo run -p sigil-harness --bin repin -- [--aeon DIR] [--check] [--verbose]
//! ```
//!
//! Default mode rewrites `crates/sigil-harness/src/pins.rs` when any pin
//! moved, printing every change as `name: old → new (Δ)`, the union of the
//! changed pins' `tests` lists (the rerun hint), and the engine.inc org
//! paste-blocks for the changed gated regions (D-T10.4/D-T10.7). `--check`
//! regenerates, diffs, and exits nonzero on drift WITHOUT writing (CI /
//! staleness mode). `--verbose` additionally prints every gated region's
//! paste block even when clean.
//!
//! The aeon tree comes from `--aeon`, else `AEON_DIR`, else the sibling
//! default. Only the typing is automated — the strict suite still
//! independently verifies bytes, and engine.inc is never edited (the block
//! is printed for pasting).

use std::path::PathBuf;
use std::process::ExitCode;

use sigil_harness::repin::{
    assert_listing_matches_rom, diff_pins, load_manifest, parse_listing, render, resolve,
    strip_provenance, Provenance,
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

    let plain_path = aeon.join("s4.lst");
    let debug_path = aeon.join("s4.debug.lst");
    let read = |p: &PathBuf| {
        std::fs::read_to_string(p).map_err(|e| format!("cannot read {}: {e}", p.display()))
    };
    let (plain_txt, debug_txt) = match (read(&plain_path), read(&debug_path)) {
        (Ok(p), Ok(d)) => (p, d),
        (Err(e), _) | (_, Err(e)) => return fail(&e),
    };
    let plain = match parse_listing(&plain_txt) {
        Ok(l) => l,
        Err(e) => return fail(&format!("{}: {e}", plain_path.display())),
    };
    let debug = match parse_listing(&debug_txt) {
        Ok(l) => l,
        Err(e) => return fail(&format!("{}: {e}", debug_path.display())),
    };

    // ── Listing freshness proof (D-T10.x hardening) ──
    // repin derives every pin from the .lst; a .lst that is STALE relative to its
    // .bin (e.g. the build recipe refreshed s4.debug.bin but not s4.debug.lst)
    // silently repins to phantom addresses. Cross-check each listing's emitted
    // bytes against its ROM before trusting the pins; warn first on mtime skew.
    for (lst_path, bin_name, txt, end_addr, shape) in [
        (&plain_path, "s4.bin", &plain_txt, plain.end_addr, "plain"),
        (&debug_path, "s4.debug.bin", &debug_txt, debug.end_addr, "debug"),
    ] {
        let bin_path = aeon.join(bin_name);
        let rom = match std::fs::read(&bin_path) {
            Ok(r) => r,
            Err(e) => return fail(&format!("cannot read {} for freshness check: {e}", bin_path.display())),
        };
        if let (Ok(lm), Ok(bm)) = (
            std::fs::metadata(lst_path).and_then(|m| m.modified()),
            std::fs::metadata(&bin_path).and_then(|m| m.modified()),
        ) {
            if lm < bm {
                eprintln!(
                    "warning: {} is older than {} — the listing may be stale (build recipe must \
                     copy the .lst alongside the .bin); verifying bytes…",
                    lst_path.display(),
                    bin_path.display()
                );
            }
        }
        if let Err(e) = assert_listing_matches_rom(txt, &rom, end_addr, shape) {
            return fail(&e);
        }
    }

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
        plain_path: plain_path.display().to_string(),
        debug_path: debug_path.display().to_string(),
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
        println!("engine.inc / main.asm gate resume orgs (paste into the else-arms):");
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
