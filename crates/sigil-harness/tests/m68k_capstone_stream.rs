//! The capstone differential over the FULL emitted 68000 stream.
//!
//! The opcode-space sweep (`sigil-isa/tests/m68k_capstone_differential.rs`)
//! covers every instruction SHAPE but pads its extension words with fixed
//! patterns, so the displacements, immediates and absolute addresses it
//! compares are the two patterns it chose. This pass supplies the other half:
//! the operand VALUES the shipped games actually compile to — real branch
//! displacements, real `(d16,An)` offsets both signs, real long immediates,
//! real absolute addresses — read back by an independent disassembler.
//!
//! The corpus is the encode-capture tap, exactly as in
//! `m68k_roundtrip_stream.rs`: every shipped shape builds inside a
//! `m68k::capture` session and every `(Instruction, bytes)` pair the build
//! encodes is collected. Only the ORACLE differs — that pass asks sigil's own
//! decoder, this one asks capstone.
//!
//! # Padding, and the `6x00` placeholder
//!
//! Each captured byte string is zero-padded to `PAD_LEN` before both sides read
//! it, so a length disagreement shows up as a length disagreement instead of
//! capstone running out of input and reporting "not an instruction". One
//! consequence is deliberate: the backend's pre-link `.s`-branch placeholder is
//! the 2-byte string `6x00`, and padded it reads as the 4-byte word form on
//! BOTH sides. That is the right subject — `6x00` alone is not an instruction
//! any ROM keeps (the linker patches it, and rejects a resolved displacement of
//! 0 outright), so comparing the padded word form compares something real.
//!
//! # This file must stay single-test
//!
//! The capture tap is process-global, so any encode this binary performs
//! outside the live session's build lands in that session's buffer as if the
//! build emitted it. A second test here — even one in another thread — would
//! contaminate the corpus.

#[path = "../../sigil-isa/tests/support/capstone_diff.rs"]
mod capstone_diff;

use capstone_diff::{
    capstone_or_skip, compare, exclusions, report, Accounting, Cap, Disagreement, PAD_LEN,
};
use sigil_harness::native;
use sigil_harness::test_support::reference_tree;
use sigil_isa::m68k::capture::CaptureSession;
use std::collections::BTreeMap;

#[test]
fn every_emitted_m68k_instruction_agrees_with_capstone() {
    // Skip-green without the aeon reference; panic under SIGIL_STRICT_GATE=1.
    // `vblank.emp` is a shape source every shipped shape's build reads.
    let Some(aeon) = reference_tree(&["engine/system/vblank.emp"]) else { return };

    // Unique padded buffers, each remembered with one shape that emitted it so
    // a failure names where to look.
    let mut buffers: BTreeMap<[u8; PAD_LEN], &'static str> = BTreeMap::new();
    let mut per_shape: Vec<(&'static str, usize)> = Vec::new();

    let session = CaptureSession::begin();
    for (label, profile) in native::shipped_shapes() {
        // Discard anything encoded outside this shape's build (prior residue).
        session.drain();
        let rom = native::build_rom_chained(&aeon, &profile)
            .unwrap_or_else(|e| panic!("shape `{label}`: build failed: {e}"));
        assert!(!rom.is_empty(), "shape `{label}`: empty ROM");
        let pairs = session.drain();
        assert!(
            !pairs.is_empty(),
            "shape `{label}` captured 0 instructions — the capture tap is disconnected \
             (or the build encoded nothing), which must never read as a pass"
        );
        for (_, bytes) in &pairs {
            assert!(
                bytes.len() <= PAD_LEN,
                "shape `{label}`: encoded {} bytes, longer than the {PAD_LEN}-byte buffer",
                bytes.len()
            );
            let mut buf = [0u8; PAD_LEN];
            buf[..bytes.len()].copy_from_slice(bytes);
            buffers.entry(buf).or_insert(label);
        }
        per_shape.push((label, pairs.len()));
    }
    drop(session);

    println!("per-shape captured m68k instructions:");
    for (label, n) in &per_shape {
        println!("  {label:>14}: {n}");
    }
    println!("distinct padded byte strings across all shapes: {}", buffers.len());

    // Derived floor, not a pinned measurement: `ALL_FAMILY_NAMES` has 62 rows
    // and the round-trip stream pass proves 61 of them appear in this same
    // corpus, so a run offering capstone fewer distinct byte strings than there
    // are families has lost the stream rather than found a smaller one.
    assert!(
        buffers.len() >= sigil_isa::m68k::ALL_FAMILY_NAMES.len(),
        "only {} distinct byte strings captured — fewer than the {} encodable families, \
         so the capture lost the stream",
        buffers.len(),
        sigil_isa::m68k::ALL_FAMILY_NAMES.len()
    );

    let keys: Vec<[u8; PAD_LEN]> = buffers.keys().copied().collect();
    let stdin: String = keys
        .iter()
        .map(|b| b.iter().map(|x| format!("{x:02X}")).collect::<String>() + "\n")
        .collect();
    let Some(recs) = capstone_or_skip("bytes", &[], Some(stdin)) else { return };
    assert_eq!(
        recs.len(),
        keys.len(),
        "capstone answered for {} of {} byte strings",
        recs.len(),
        keys.len()
    );

    let excl = exclusions();
    let mut hits = vec![0usize; excl.len()];
    let mut bad: Vec<Disagreement> = Vec::new();
    let mut cap_decodable = 0usize;

    for (buf, (key, cap)) in keys.iter().zip(&recs) {
        let word = u16::from_be_bytes([buf[0], buf[1]]);
        if matches!(cap, Cap::Ok { .. }) {
            cap_decodable += 1;
        }
        let shape = buffers[buf];
        let named = format!("{key} (first emitted by `{shape}`)");
        if let Some(d) = compare(&named, word, buf, cap, &excl, &mut hits) {
            bad.push(d);
        }
    }

    println!("capstone decoded {cap_decodable} of {} emitted byte strings", keys.len());
    assert!(
        cap_decodable > 0,
        "capstone decoded none of the emitted stream — the oracle is not answering"
    );

    report(&excl, &hits, &bad, "emitted stream", Accounting::CountsAreInformational);
}
