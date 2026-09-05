// The `text` block in the module doc below is an asl listing pasted verbatim, and asl
// separates its columns with TABS. Those tabs are the evidence: what that comment asserts
// is what the reference assembler PRINTED, so respacing them into four spaces would
// quietly restate the claim about output asl never produced.
//
// This one is an INNER attribute, unlike the per-item waivers in `as_float_int.rs`, and
// the difference is forced rather than chosen: the listing lives in the crate-root `//!`
// doc, and Rust has no narrower scope than the module for a lint fired there. The blast
// radius is this single integration-test binary, whose only other doc lines are prose —
// it is not a crate-wide allow over library code, which would be trading a correct lint
// everywhere for one site.
#![allow(clippy::tabs_in_doc_comments)]

//! The accepted range of a WORD-SIZED IMMEDIATE, which is where the `$FFFF….`
//! game-RAM labels land.
//!
//! `s2.asm` writes `move.w #bytesToLcnt(CrossResetRAM-RAM_Start),d6` — a `.w`
//! immediate whose operand is arithmetic over labels the RAM section places at
//! `$FFFF0000`-and-up. The DIFFERENCE of two such labels is small and in range;
//! the labels themselves are not. Which of those two a front end computes is
//! invisible in a diagnostic count when it gets the answer wrong in the
//! PERMISSIVE direction — accepting `#$FFFFF700` emits four bytes and no
//! complaint — so this file asserts acceptance and refusal as a pair, at the
//! boundary, against what asl actually does.
//!
//! Every expectation is read off the listing of `asl` 1.42 Beta Bld 212 — the
//! binary committed at `s2disasm/build_tools/Linux-x86_64/asl` — for the
//! identical source text. Probe `wrange.asm`, committed under
//! `docs/superpowers/notes/2026-09-04-as-end-probes/`:
//!
//! ```text
//!        4/       0 : 303C FFFF           	move.w	#65535,d0
//!        5/       4 : 303C 8000           	move.w	#-32768,d0
//! > > > wrange.asm(6):10: error: range overflow
//!        6/       8 : 303C 55F5           	move.w	#-32769,d0
//! > > > wrange.asm(7):10: error: range overflow
//!        7/       C : 303C 55F5           	move.w	#65536,d0
//! > > > wrange.asm(8):10: error: range overflow
//!        8/      10 : 303C 55F5           	move.w	#-65536,d0
//! > > > wrange.asm(9):10: error: range overflow
//!        9/      14 : 303C 55F5           	move.w	#$FFFFF700,d0
//! ```
//!
//! **⚠ THE FOUR `303C 55F5` WORDS ABOVE ARE UNINITIALIZED MEMORY, NOT ASL'S
//! ANSWER, AND THEY DO NOT REPRODUCE** *(2026-09-05)*. The listing was taken
//! with `s2disasm/build_tools/Linux-x86_64/asl` (md5
//! `0dee1f98e6480a4783d27ffd8b90896f`), which substitutes an UNINITIALIZED READ
//! for any operand it declined to give a value. The word is different on every
//! run — `5602`, `55B1`, `5655`, `557F` on four consecutive ones — and the
//! `55F5` here is one draw from that, frozen into a comment as though it were
//! output. Both builds print `AS 1.42 Beta [Bld 212]` verbatim, which is why the
//! binary is cited by digest here and not by version.
//!
//! **AND THE REFERENCE BUILD DOES NOT ANSWER THESE LINES EITHER** *(amended
//! 2026-09-05)*. This paragraph used to end by saying that `s1disasm`'s build
//! (md5 `61e672562465725a8c102288a7da9098`) "prints `303C 8000` on all four
//! lines, every run" — which corrected the varying build's artifact by
//! enshrining the reference build's. That `8000` is **line 5 leaking downward**:
//! line 5 is `move.w #-32768,d0`, in range, ACCEPTED, and legitimately `$8000`,
//! and the four refused lines echo the last value asl computed. The control is
//! `wcarry.asm` beside `wrange.asm` — the same file with line 5's accepted value
//! changed to `$1234` and lines 6-9 untouched — where all four then read
//! `303C 1234`. `wcarry0.asm`, with no accepted immediate above the refused
//! ones, reads `0000`: the slot's initial state, not a policy.
//!
//! So the reference build is stable here and still not answering. That is the
//! more freezable of the two failure modes, because re-running it confirms it.
//!
//! **The rule and every assertion below are UNAFFECTED, and that is measured
//! rather than argued.** The four `> > > range overflow` diagnostics come back
//! identical from both builds, at the same lines; the two ACCEPTED rows
//! (`303C FFFF` on line 4, `303C 8000` on line 5) are identical too — those two
//! ARE answers, and line 5's is the one the refused lines below it go on to
//! echo. The tests here assert acceptance and refusal, never the byte column of
//! a refused line — nothing reads the substituted word on either build. The
//! stale listing is kept rather than swapped so the supersession is visible
//! instead of silent.
//!
//! asl's word immediate spans `-32768..=65535`: the signed floor and the
//! unsigned ceiling, both inclusive. `-65536` is the value the booked
//! `AS-WORD-IMM-RAM-LABEL` row was named after — `$FFFF0000` read as a signed
//! 32-bit integer — and asl refuses it exactly as sigil does. The 34 corpus
//! instances were never this rule disagreeing; they were the RAM labels
//! carrying wrong values upstream, so a difference that should have been small
//! arrived as a whole `$FFFF….` address.
//!
//! The BYTES for the in-range shape (including `lea (RamLabel).w,a6`, the other
//! word-sized reference to a `$FFFF….` label) are pinned as the asl-minted
//! `as_word_immediate_against_ffff_ram_labels` golden block in
//! `tests/snippets_golden.txt`.

use sigil_frontend_as::{assemble, Options};

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\tphase 0\n";

fn accepted(body: &str) -> bool {
    assemble(&format!("{HEAD}{body}\n"), &Options::default()).is_ok()
}

/// The two values asl assembles (`303C FFFF` and `303C 8000`).
#[test]
fn word_immediate_accepts_the_signed_floor_and_the_unsigned_ceiling() {
    assert!(
        accepted("\tmove.w\t#65535,d0"),
        "asl emits `303C FFFF` for `move.w #65535,d0` — the unsigned ceiling is in range"
    );
    assert!(
        accepted("\tmove.w\t#-32768,d0"),
        "asl emits `303C 8000` for `move.w #-32768,d0` — the signed floor is in range"
    );
}

/// The four values asl refuses with `range overflow` — one step past each end,
/// and the two `$FFFF….` forms the RAM labels produce.
#[test]
fn word_immediate_refuses_exactly_where_asl_reports_range_overflow() {
    for body in [
        "\tmove.w\t#-32769,d0",
        "\tmove.w\t#65536,d0",
        "\tmove.w\t#-65536,d0",
        "\tmove.w\t#$FFFFF700,d0",
    ] {
        assert!(
            !accepted(body),
            "asl reports `range overflow` for `{}` — accepting it emits four bytes and \
             no complaint, which no diagnostic count can see",
            body.trim()
        );
    }
}

/// The shape the booked row actually named: a `.w` immediate over a DIFFERENCE
/// of two `$FFFF….` RAM labels is in range and must assemble. Asserted next to
/// the refusals so a front end cannot pass this file by refusing everything.
#[test]
fn word_immediate_accepts_a_difference_of_two_ffff_ram_labels() {
    assert!(
        accepted(
            "RAM_Start\t= $FFFF0000\n\
             CrossResetRAM\t= $FFFF8000\n\
             \tmove.w\t#((CrossResetRAM-RAM_Start)/4)-1,d6"
        ),
        "`move.w #bytesToLcnt(CrossResetRAM-RAM_Start),d6` is the s2.asm:413 shape; asl \
         emits `3C3C 1FFF`"
    );
}
