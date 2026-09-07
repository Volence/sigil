//! seam-2 stage-2d — the SFX window-pointer HEAD, co-linked and proven against
//! the reference ROM (the twins-present dual proof, both shapes).
//!
//! `sigil_harness::seam2::emit_sfx_body_and_head` CO-LINKS the REAL `sfx_bank.emp`
//! (the blob table + the `SFX_WIN_*` equ layer) with the REAL
//! `sfx_blob_win_tab.emp` (the phased head) in one link, per SHAPE: the head's
//! `dc.w SFX_WIN_NN` cells resolve as CROSS-MODULE link symbols against
//! `sfx_bank.emp`'s `SFX_WIN_*` equs (which fold same-module from
//! `winptr(Sfx_NN)`). The `SFX_WIN_*` names live ONCE, at the producer that owns
//! the SFX block placement.
//!
//! This gate proves the co-linked head is BYTE-IDENTICAL to the `SfxBlobWinTab`
//! slice of the assembled reference ROM (at the LMA `sound_layout` derives,
//! `sfx_win_tab_lma`; `SFX_WIN_TAB_LEN` bytes), the
//! "twins present, both paths byte-identical" dual proof that must be GREEN before
//! `sfx_blob_win_tab.asm` (+ the SFX body `.asm`s) can be retired.
//!
//! SHAPE-DEPENDENT (unlike the DAC head): the SFX block sits AFTER the
//! shape-dependent song tables (`sfx_bank_lma_plain` / `sfx_bank_lma_debug`), so every
//! real cell (`winptr(Sfx_NN)`) shifts with `__DEBUG__`. So this gate co-links
//! per shape and asserts each against ITS OWN reference ROM.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_sfx_head_colink
//! ```

use sigil_harness::seam2::{
    emit_sfx_artifacts, emit_sfx_body_and_head, emit_sfx_body_and_head_doctored, sound_layout,
    SoundLayout,
};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}
#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}
/// The FROZEN golden slice comparand (the asl-witnessed reference), NOT the live
/// tree ROM — post-flip `aeon/s4.bin` is itself sigil-built (row-91 bar b).
fn golden(name: &str) -> Vec<u8> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../sigil-harness/golden/{name}"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("read golden {}: {e}", path.display()))
}

/// The reference SFX-block body window per shape, read off `sound_layout`
/// (`sfx_bank_lma_plain` / `sfx_bank_lma_debug`), the one place those two addresses
/// are derived; `sound_layout_derives_the_frozen_addresses` in
/// seam2_layout_derivation.rs is the test that pins them against literals. One pin,
/// one place.
fn body_window(layout: &SoundLayout, debug: bool) -> (&'static str, usize) {
    if debug {
        ("s4.debug.bin", layout.sfx_bank_lma_debug as usize)
    } else {
        ("s4.bin", layout.sfx_bank_lma_plain as usize)
    }
}

const SFX_WIN_TAB_LEN: usize = 274; // 137 dense ids ($33..=$BB) × 2 bytes
/// The co-linked `sfx_bank` body length, in bytes. A TRIPWIRE: it does not feed the
/// co-link, it asserts that the emitted body is the size this gate was last taught.
///
/// Re-pin it whenever the SFX set changes. It tracks `pins::SFX_BANK_BLOB`'s `plain_len`
/// exactly (2284 == 0x8EC today), which is the cheap way to check it: if those two
/// disagree, this constant is stale, not the emitter.
const SFX_BODY_LEN: usize = 2284;

/// THE HEAD BYTE GATE: the co-linked `SfxBlobWinTab` == the reference ROM slice
/// at `sfx_win_tab_lma`, in BOTH shapes (each vs its own ROM; the head is
/// shape-dependent). The body from the SAME co-link also matches, proving the
/// pair is consistent.
#[test]
fn colinked_sfx_head_matches_the_reference_rom_slice_both_shapes() {
    if !strict_gate() {
        eprintln!("skip: seam2_sfx_head_colink not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let layout = sound_layout(&aeon).expect("sound_layout derives the SFX LMAs");
    let win_lma = layout.sfx_win_tab_lma;
    for (debug, shape) in [(false, "plain"), (true, "debug")] {
        let out = emit_sfx_body_and_head(&aeon, debug).expect("emit_sfx_body_and_head co-links");
        assert_eq!(out.head.len(), SFX_WIN_TAB_LEN, "SfxBlobWinTab is 137 × 2 = 274 bytes");
        assert_eq!(out.body.len(), SFX_BODY_LEN, "sfx_bank body is {SFX_BODY_LEN} bytes");

        let rom = golden(if debug { "s4.debug.bin" } else { "s4.bin" });
        let lo = win_lma as usize;
        let head_ref = &rom[lo..lo + SFX_WIN_TAB_LEN];
        if let Some(i) = (0..out.head.len()).find(|&i| out.head[i] != head_ref[i]) {
            let e = i & !1; // word-aligned context
            panic!(
                "co-linked SFX head differs from {shape} reference @ byte {i:#x}: \
                 emp {:#04x} vs rom {:#04x}\n  emp[{e:#x}..]: {:02x?}\n  rom[{e:#x}..]: {:02x?}",
                out.head[i], head_ref[i],
                &out.head[e..(e + 8).min(out.head.len())],
                &head_ref[e..(e + 8).min(head_ref.len())],
            );
        }
        assert_eq!(
            out.head, head_ref,
            "co-linked SfxBlobWinTab must equal the {shape} reference @ {win_lma:#X} (SfxBlobWinTab)"
        );

        let (rom_name, base) = body_window(&layout, debug);
        assert_eq!(rom_name, if debug { "s4.debug.bin" } else { "s4.bin" });
        let body_ref = &rom[base..base + SFX_BODY_LEN];
        assert_eq!(out.body, body_ref, "co-linked sfx_bank body must equal the {shape} reference");
    }
}

/// t24 NON-VACUITY control (row-91 bar c): a doctored composition, the SFX block
/// co-linked `$100` bytes higher (still inside the SFX block's own bank, so the
/// body co-residency ensures stay green), must make the head DIVERGE from the golden
/// slice, because every
/// `SFX_WIN_NN = winptr(Sfx_NN)` re-folds from the moved blobs. The head byte gate
/// is vacuous if a moved SFX block still matches.
#[test]
fn sfx_head_diverges_when_block_moved() {
    if !strict_gate() {
        eprintln!("skip: seam2_sfx_head_colink not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let layout = sound_layout(&aeon).expect("sound_layout");
    // +$100 must keep the block inside its own `$8000` bank (`bankid` unchanged), so
    // only the window pointers shift and the body's co-residency guards do not fire.
    // Checked rather than assumed: the bank is wherever the map puts it.
    let moved = layout.sfx_bank_lma_plain + 0x100;
    let bankid = |lma: u32| (lma & 0x7F_8000) >> 15;
    assert_eq!(
        bankid(moved),
        bankid(layout.sfx_bank_lma_plain),
        "the +$100 control must stay inside the SFX block's own bank, otherwise the body \
         co-residency ensures fire instead of the head diverging"
    );
    let doctored = emit_sfx_body_and_head_doctored(&aeon, false, Some(moved))
        .expect("doctored co-link (same bank)");
    let rom = golden("s4.bin");
    let lo = layout.sfx_win_tab_lma as usize;
    let head_ref = &rom[lo..lo + SFX_WIN_TAB_LEN];
    assert_ne!(
        doctored.head, head_ref,
        "the SFX head gate is vacuous if a moved SFX block still matches the golden slice"
    );
}

/// Determinism: the co-link is byte-stable across runs.
#[test]
fn colink_is_deterministic() {
    if !strict_gate() {
        eprintln!("skip: seam2_sfx_head_colink not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    for debug in [false, true] {
        let a = emit_sfx_body_and_head(&aeon, debug).expect("emit 1");
        let b = emit_sfx_body_and_head(&aeon, debug).expect("emit 2");
        assert_eq!(a.head, b.head, "head emit must be deterministic");
        assert_eq!(a.body, b.body, "body emit must be deterministic");
    }
}

/// The EMITTER binary's SFX artifacts (`emit_sfx_artifacts`, driven by the
/// `emit_sound_blob` bin the real build runs) are written to disk and equal the
/// in-memory co-link — the build's BINCLUDE inputs match the proven reference.
#[test]
fn emit_sfx_artifacts_writes_reference_bins() {
    if !strict_gate() {
        eprintln!("skip: seam2_sfx_head_colink not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let dir = tempfile::tempdir().expect("tempdir");
    emit_sfx_artifacts(&aeon, dir.path()).expect("emit_sfx_artifacts writes the 4 .bins");
    for (debug, body_name, head_name) in [
        (false, "sfx_bank.bin", "sfx_blob_win_tab.bin"),
        (true, "sfx_bank_debug.bin", "sfx_blob_win_tab_debug.bin"),
    ] {
        let mem = emit_sfx_body_and_head(&aeon, debug).expect("co-link");
        let body = std::fs::read(dir.path().join(body_name)).unwrap_or_else(|e| panic!("read {body_name}: {e}"));
        let head = std::fs::read(dir.path().join(head_name)).unwrap_or_else(|e| panic!("read {head_name}: {e}"));
        assert_eq!(body, mem.body, "emitted {body_name} must equal the in-memory co-link (== reference)");
        assert_eq!(head, mem.head, "emitted {head_name} must equal the in-memory co-link (== reference)");
    }
}
