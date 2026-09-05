//! Exhaustive opcode-space sweep: decoder legality == encoder legality,
//! permanently.
//!
//! For every one of the 65,536 opcode words (padded with zero extension
//! words), `decode_one` must either error or produce an instruction whose
//! `encode` reproduces exactly the bytes the decoder consumed. This closes
//! the one hole the per-form suites cannot see: a decoder `EaSet` row
//! loosened to a SUPERSET of the encoder's would accept words the encoder
//! can never emit, and every encode-first test would stay green — here, the
//! decode succeeds, the re-encode rejects (or moves a field), and the sweep
//! names the word.
//!
//! Zero-padded extension words are total for this purpose: every extension
//! field the emitted set uses (displacements, immediates, masks, brief
//! extensions) admits the all-zero word, so padding never blocks a decodable
//! opcode word, and any decode-side validation of forbidden extension bits
//! fires on real nonzero patterns, which the per-form tests cover.
//!
//! There are NO exceptions: every word where decode succeeds re-encodes
//! byte-for-byte. If a change makes that impossible for some class, the
//! class belongs HERE as a named, justified carve-out — never a silent skip.

use sigil_isa::m68k::encode;
use sigil_isa::m68k_decode::decode_one;

#[test]
fn every_decodable_word_reencodes_to_the_consumed_bytes() {
    // Longest emitted form is 10 bytes (opcode + four extension words, e.g.
    // `move.l #imm32,(xxx).l`); pad generously so consumption is never
    // truncated by the harness itself.
    let mut buf = [0u8; 14];
    let mut failures: Vec<String> = Vec::new();
    let mut decodable = 0usize;

    for w in 0..=0xFFFFu16 {
        buf[..2].copy_from_slice(&w.to_be_bytes());
        let Ok((inst, used)) = decode_one(&buf) else { continue };
        decodable += 1;
        match encode(&inst) {
            Ok(bytes) if bytes == buf[..used] => {}
            Ok(bytes) => failures.push(format!(
                "word {w:04X}: decodes to {inst:?} (consumed {:02X?}) but re-encodes to {bytes:02X?}",
                &buf[..used]
            )),
            Err(e) => failures.push(format!(
                "word {w:04X}: decodes to {inst:?} but the encoder REJECTS it ({e}), \
                 the decoder accepts a word the encoder cannot emit"
            )),
        }
    }

    if !failures.is_empty() {
        let shown = failures.iter().take(25).cloned().collect::<Vec<_>>().join("\n");
        panic!(
            "{} of {decodable} decodable words failed the re-encode (first {} shown):\n{shown}",
            failures.len(),
            failures.len().min(25)
        );
    }
    // Derived floor, not a pinned measurement: the 2048 `moveq` words alone
    // (0111 rrr 0 dddddddd) are all decodable, so a sweep that decodes fewer
    // than that has lost whole instruction lines.
    assert!(
        decodable >= 2048,
        "only {decodable} of 65536 words decoded, the decoder lost whole lines"
    );
    println!("opcode sweep: {decodable} of 65536 words decodable, all re-encode byte-exact");
}
