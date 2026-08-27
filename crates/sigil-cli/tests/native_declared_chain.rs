//! Flip Stage 2 · S1.2 — THE DECLARED-ORDER, COMPUTED-BASE CHAINER GATE.
//!
//! `native_chained_resume` proved that re-tagging the exactly-abutting sections as
//! `Chained` reproduces asl — but it left every non-abutting section PINNED at its
//! baked lma, so most of the layout still leaned on baked addresses. This gate proves
//! the FULL generalization (`native::build_native_rom_chained`): EVERY section's base
//! is computed by declared-order chaining, with only the genuine org anchors (object
//! bank, phased sound banks) keeping a declared base.
//!
//! THE REFINEMENT IT LOCKS IN: pure content-size chaining does NOT reproduce asl — the
//! link relaxer settles at a tighter fixpoint than asl (a boundary branch relaxes
//! short; first seen as −2 at `hblank`). So the chainer reserves each section's DECLARED
//! SIZE (its exact asl per-region span), which anchors relaxation to asl's widths while
//! the base stays computed. This gate is the regression proof that the mechanism holds
//! byte-for-byte, BOTH shapes, over the assembled bar (header-neutral).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_declared_chain
//! ```
use sigil_harness::{native, pins};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}
fn read_ref(aeon: &Path, name: &str) -> Option<Vec<u8>> {
    match std::fs::read(aeon.join(name)) {
        Ok(b) => Some(b),
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but reference missing: {name}");
            }
            None
        }
    }
}

// The whole-ROM native build touches the shared `engine/sound/generated` dir.
static GEN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Header-neutral diff count over the assembled bar (zero the checksum `$18E` and the
/// ROM-end pointer `$1A4` — the fields the golden computes over the full appendix'd
/// file, which the assembled-bar comparison legitimately excludes).
fn header_neutral_diffs(a: &[u8], b: &[u8], n: usize) -> usize {
    let mut a = a[..n].to_vec();
    let mut b = b[..n].to_vec();
    for buf in [&mut a, &mut b] {
        buf[0x18e..0x190].fill(0);
        buf[0x1a4..0x1a8].fill(0);
    }
    (0..n).filter(|&i| a[i] != b[i]).count()
}

/// (PLAIN) every base computed by declared-order chaining == canonical asl `s4.bin`.
#[test]
fn declared_chain_plain() {
    let aeon = aeon_dir();
    let Some(refrom) = read_ref(&aeon, "s4.bin") else { return };
    let _g = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let rom = native::build_native_rom_chained(&aeon, false).unwrap_or_else(|e| panic!("{e}"));
    let n = pins::ASSEMBLED_LEN;
    assert_eq!(
        header_neutral_diffs(&rom, &refrom, n),
        0,
        "declared-order computed-base chain (plain) diverged from s4.bin over [0,{n:#x})"
    );
}

/// (DEBUG) every base computed by declared-order chaining == canonical asl `s4.debug.bin`.
#[test]
fn declared_chain_debug() {
    let aeon = aeon_dir();
    let Some(refrom) = read_ref(&aeon, "s4.debug.bin") else { return };
    let _g = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let rom = native::build_native_rom_chained(&aeon, true).unwrap_or_else(|e| panic!("{e}"));
    let n = pins::DEBUG_ASSEMBLED_LEN;
    assert_eq!(
        header_neutral_diffs(&rom, &refrom, n),
        0,
        "declared-order computed-base chain (debug) diverged from s4.debug.bin over [0,{n:#x})"
    );
}
