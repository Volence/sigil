//! The capstone differential over the whole 68000 opcode space.
//!
//! Companion to `m68k_opcode_sweep.rs`, walking the same 65,536 words with a
//! different oracle. That sweep's oracle is sigil's own `encode`, so it proves
//! *decoder ⊆ encoder* and cannot see a defect the two halves share; this one
//! asks capstone, which shares no lineage with sigil. The comparison, the
//! canonical form and the named exclusions all live in
//! [`support/capstone_diff.rs`](support/capstone_diff.rs) — read that file for
//! what "disagreement" means here and why each exclusion is capstone's error
//! and not sigil's.
//!
//! # Extension-word padding, and why more than one pad
//!
//! A sweep pads each opcode word with extension words it chooses. The
//! all-zero padding the encoder-oracle sweep uses makes every displacement,
//! immediate and absolute address zero, so it compares instruction SHAPE
//! thoroughly and operand VALUES not at all — a sign-extension or byte-order
//! error in a displacement field is invisible to it.
//!
//! So the space is walked twice, with pads picked from what each extension-word
//! consumer requires rather than at random:
//!
//! | pad word | what it exercises | what it costs |
//! |---|---|---|
//! | `$0000` | every form; the exact class sizes | all values zero |
//! | `$00FF` | `(d8,An,Xn)` displacement `−1` (the disp8 sign path), `$00FF` displacements and absolutes, `$00FF00FF` longs, byte immediate `$FF` | nothing — this pad is legal in every extension-word consumer |
//!
//! The constraints are the decoder's own: `imm_ext` rejects a byte immediate
//! whose high byte is nonzero, and `brief` rejects an extension word with any of
//! bits 10–8 set, so a pad must keep `ext & $0700 == 0` to reach the indexed
//! forms at all.
//!
//! A NEGATIVE `(d16,An)` displacement is not reachable this way and is
//! deliberately not attempted: any pad word with its high bit set also has a
//! nonzero high byte, which lands on an open question about the static bit-ops'
//! bit-number extension word rather than on displacement handling. That
//! coverage comes from the emitted-stream pass
//! (`sigil-harness/tests/m68k_capstone_stream.rs`), whose displacements are
//! whatever the shipped games actually compile to.
//!
//! Only the `$0000` pass asserts the exclusions' exact class sizes: a nonzero
//! pad makes some words undecodable, which legitimately shrinks a class, and
//! pinning a count there would pin a measurement instead of a derivation. Both
//! passes fail on any UNEXCUSED disagreement.

#[path = "support/capstone_diff.rs"]
mod capstone_diff;

use capstone_diff::{
    capstone_or_skip, compare, exclusions, report, Accounting, Cap, Disagreement, PAD_LEN,
};
use sigil_isa::m68k_decode::decode_one;

/// The extension-word pattern each pass pads with, and whether that pass holds
/// the exclusions to their exact class sizes. See the module docs for the
/// derivation of each pad.
const PADS: &[(u16, Accounting)] = &[
    (0x0000, Accounting::ExactClassSizes),
    (0x00FF, Accounting::CountsAreInformational),
];

#[test]
fn opcode_sweep_agrees_with_capstone() {
    for (pad, mode) in PADS {
        run_pass(*pad, *mode, &format!("opcode sweep, pad ${pad:04X}"));
    }
}

fn run_pass(pad_word: u16, mode: Accounting, what: &str) {
    let pad = pad_word.to_be_bytes();
    let arg = format!("--pad2=0x{pad_word:04X}");
    let Some(recs) = capstone_or_skip("sweep", &[arg], None) else { return };
    assert_eq!(recs.len(), 0x10000, "capstone dump covered {} words, not 65536", recs.len());

    let excl = exclusions();
    let mut hits = vec![0usize; excl.len()];
    let mut bad: Vec<Disagreement> = Vec::new();
    let mut sigil_decodable = 0usize;
    let mut cap_decodable = 0usize;

    let mut buf = [0u8; PAD_LEN];
    for (w, (key, cap)) in recs.iter().enumerate() {
        let w = w as u16;
        assert_eq!(key, &format!("{w:04X}"), "capstone dump out of order at {key}");
        buf[..2].copy_from_slice(&w.to_be_bytes());
        for (i, b) in buf[2..].iter_mut().enumerate() {
            *b = pad[i % 2];
        }
        if decode_one(&buf).is_ok() {
            sigil_decodable += 1;
        }
        if matches!(cap, Cap::Ok { .. }) {
            cap_decodable += 1;
        }
        if let Some(d) = compare(key, w, &buf, cap, &excl, &mut hits) {
            bad.push(d);
        }
    }

    println!("{what}: sigil decodes {sigil_decodable} of 65536 words, capstone decodes {cap_decodable}");
    // Derived floor, not a pinned measurement — the same one the encoder-oracle
    // sweep uses. The 2048 `moveq` words (`0111 rrr 0 dddddddd`) carry no
    // extension words, so they are decodable under EVERY pad; a pass below that
    // has lost whole instruction lines and is comparing nothing.
    assert!(
        sigil_decodable >= 2048,
        "{what}: only {sigil_decodable} words decoded by sigil, the decoder lost whole lines"
    );
    assert!(
        cap_decodable >= 2048,
        "{what}: capstone decoded only {cap_decodable} words, the oracle is not answering"
    );

    report(&excl, &hits, &bad, what, mode);
}
