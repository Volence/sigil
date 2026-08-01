//! Stage-3 P4b — THE OBJECT-BANK BUDGET AS A MAP-REGION CHECK.
//!
//! `sigil.map.toml`'s `object_bank` region ($10000, size $10000) is the map-owned
//! successor to `engine.inc`'s deleted `if * > $20000 / error`. Every native build runs
//! `native::check_object_bank_budget` (the declared budget cursor — the data-region head
//! `DeformTable_Zero`, K4 inc-6B's successor to the retired `__BUDGET_DATA` marker — vs the
//! region cap) before `emit_rom`. This gate proves the check is (a) SATISFIED on the real
//! canonical layout with headroom, and (b) genuinely LOAD-BEARING — a doctored (shrunk)
//! budget on the SAME resolved layout fails loudly (t24).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_object_bank_budget
//! ```
use sigil_harness::{map_placement, native};
use std::path::PathBuf;

/// The sigil crate's `sigil.map.toml` placement facts (the object-bank budget cursor).
fn placement_map() -> map_placement::PlacementMap {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    map_placement::load_placement_map(&std::fs::read_to_string(&p).unwrap())
        .unwrap_or_else(|e| panic!("load placement {}: {e}", p.display()))
}

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}
fn have_aeon(aeon: &PathBuf) -> bool {
    if aeon.join("s4.bin").exists() {
        return true;
    }
    if strict_gate() {
        panic!("SIGIL_STRICT_GATE set but aeon tree missing at {}", aeon.display());
    }
    eprintln!("skip: aeon tree not present (set AEON_DIR)");
    false
}

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The real canonical layout satisfies the map's object-bank budget with headroom — the
/// `__BUDGET_DATA` cursor is well inside `[$10000, $20000)`. Reports the used bytes.
#[test]
fn canonical_object_bank_within_budget() {
    let aeon = aeon_dir();
    if !have_aeon(&aeon) {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let resolved = native::resolve_canonical_sections(&aeon, false).unwrap_or_else(|e| panic!("{e}"));
    let map = native::project_memory_map().unwrap_or_else(|e| panic!("{e}"));
    let pmap = placement_map();
    let used =
        native::check_object_bank_budget(&resolved, &map, &pmap).unwrap_or_else(|e| panic!("{e}"));
    eprintln!("object bank used = {used:#x} / 0x10000");
    assert!(used > 0, "object bank cursor not found (is the budget cursor label present?)");
    assert!(used < 0x10000, "object bank over budget on the real layout: {used:#x}");
}

/// t24 — the budget is load-bearing: doctoring the map's `object_bank` region to a tiny
/// cap makes the SAME resolved layout fail the check (the deleted AS `error` guard's job,
/// now map-owned). Proves a real object-bank overflow would fail the build, not slip past.
#[test]
fn doctored_tiny_budget_fails_the_check() {
    let aeon = aeon_dir();
    if !have_aeon(&aeon) {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let resolved = native::resolve_canonical_sections(&aeon, false).unwrap_or_else(|e| panic!("{e}"));
    let pmap = placement_map();

    // Baseline: the real map passes.
    let real = native::project_memory_map().unwrap_or_else(|e| panic!("{e}"));
    native::check_object_bank_budget(&resolved, &real, &pmap)
        .unwrap_or_else(|e| panic!("control: real map must pass, got {e}"));

    // Doctor: shrink the object_bank region to 0x100 bytes — the cursor now overflows.
    let mut doctored = real;
    let region = doctored
        .regions
        .iter_mut()
        .find(|r| r.name == "object_bank")
        .expect("map must declare object_bank");
    region.size = 0x100;
    let err = native::check_object_bank_budget(&resolved, &doctored, &pmap)
        .expect_err("t24: a 0x100 object-bank budget must overflow — the check is vacuous otherwise");
    assert!(err.contains("over by"), "expected an actionable overflow message, got: {err}");
}
