//! The split owner ruling `d-23-answered` asked for: a layout expression whose
//! size is provably one of its own inputs is refused BY NAME, everything else
//! settles with no fixed pass count, and only a run that truly oscillates is
//! reported, by the symbols still moving and the values they move between.
//!
//! ## What these tests are for, stated first
//!
//! **A detector that never fires would pass every other test in this repo.** It
//! would remove the pass cap, keep both corpora building, keep the suite green,
//! and turn `c2` below from "did not converge within 16 passes" into a
//! moving-symbol report, which is a visible improvement on its own. Nothing else
//! anywhere measures over-refusal or over-acceptance. So this file is built in
//! BOTH directions and neither half is optional:
//!
//! - `refuses_*` prove the detector FIRES when it should, and each asserts the
//!   named symbol and the named definition site, not merely that something
//!   failed.
//! - `accepts_*` prove it does NOT fire when it should not, and each asserts the
//!   exact bytes. `accepts_forward_constant_count` and
//!   `accepts_count_separated_by_an_org` are the whole false-positive surface: a
//!   cycle detector written without conjunct (d) refuses both, and both are
//!   programs that assemble here today.
//! - `oscillation_*` prove the non-settling report names values and NOT a count.
//!
//! ## The property being tested
//!
//! A layout-determining expression at point `p` (a `rept` count, a `ds` count)
//! is provably circular when, inside ONE pass: (a) it names `S`; (b) `S` is not
//! defined at `p`; (c) `S` is bound later in the same pass and the binding is
//! reached; (d) `S` is location-derived and no `org`/`phase`/`dephase`/section
//! change ran in between. Full argument in
//! `docs/superpowers/notes/2026-09-10-as-circular-split.md` section 2.
//!
//! ## Where the expected values come from
//!
//! Reference asl is `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
//! md5 `61e672562465725a8c102288a7da9098`, verified before use. The other asl in
//! the tree (`s2disasm/build_tools/.../asl`, md5
//! `0dee1f98e6480a4783d27ffd8b90896f`) was not run for any value here.
//!
//! **asl is the oracle for BEHAVIOUR and not for this POLICY**, and the two
//! split cleanly:
//!
//! - Where asl ACCEPTS (`accepts_backward_count`, `accepts_forward_constant_data`,
//!   `accepts_literal_count_with_a_derived_equ_below`), the expected bytes are
//!   asl's own listing, quoted at each test, and sigil matches them.
//! - Where asl REFUSES and sigil accepts (`accepts_forward_constant_count`,
//!   `accepts_count_separated_by_an_org`, `accepts_settling_if_condition`), asl
//!   supplies no bytes and the expectation is sigil's own, pinned here so a
//!   later change to that behaviour is a decision and not a drift. asl's refusal
//!   on all three is `expression must be evaluatable in first pass`, exit 2.
//!   These are the standing over-acceptance divergence recorded in the note's
//!   section 6; the ruling puts them in "everything else settles", so this
//!   parcel leaves them settling.
//!
//! **asl refuses every shape in this file except three.** That is the
//! containment argument behind the whole design: the set refused here is a
//! SUBSET of asl's refusal set, so no program that assembles under asl can be
//! refused by it.

use sigil_frontend_as::{assemble_root_located, Options};

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok(sigil_link::flatten(&linked, 0x00).unwrap())
        }
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

fn bytes(body: &str) -> Vec<u8> {
    match assemble(body) {
        Ok(b) => b,
        Err(d) => panic!("expected this to assemble, got diagnostics: {d:?}"),
    }
}

fn refusal(body: &str) -> String {
    match assemble(body) {
        Ok(b) => panic!("expected a refusal, got {} bytes: {b:02X?}", b.len()),
        Err(d) => d.join(" || "),
    }
}

const HEAD: &str = "\tcpu 68000\n\torg 0\n";

// ---------------------------------------------------------------------------
// FIRES WHEN IT SHOULD
// ---------------------------------------------------------------------------

/// A repeat counting by an equate that measures the repeat's own block.
///
/// `N equ End-Start` and the block is `N` bytes long, so `N = N`: every value is
/// a fixpoint and the one the assembler lands on is an artifact of what it
/// seeded with, not an answer. Sigil ACCEPTED this before the split and emitted
/// a one-byte image, which is the quietest possible wrong answer.
///
/// asl, exit 2: `c1.asm(4):7: error: expression must be evaluatable in first pass`.
#[test]
fn refuses_a_count_that_measures_its_own_block() {
    let src = format!("{HEAD}Start:\n\trept N\n\tdc.b 0\n\tendr\nEnd:\nN\tequ End-Start\n\tdc.b $FF\n");
    let d = refusal(&src);
    assert!(d.contains("circular repeat count"), "{d}");
    assert!(d.contains("`rept`"), "{d}");
    assert!(d.contains("`N`"), "{d}");
    // The definition site is named, which is the half that makes the message
    // actionable: the author is told where the other end of the loop is.
    assert!(d.contains("probe.asm(8)"), "{d}");
}

/// The same loop, one byte off a fixpoint, so it diverges instead of settling.
/// This is the shape that produced `assembly did not converge within 16 passes`,
/// which is the message the ruling exists to remove.
///
/// asl, exit 2: `c2.asm(4):7: error: expression must be evaluatable in first pass`.
#[test]
fn refuses_a_diverging_count_by_name_and_never_says_a_pass_count() {
    let src =
        format!("{HEAD}Start:\n\trept N\n\tdc.b 0\n\tendr\nEnd:\nN\tequ End-Start+1\n\tdc.b $FF\n");
    let d = refusal(&src);
    assert!(d.contains("circular repeat count"), "{d}");
    assert!(d.contains("`N`"), "{d}");
    assert!(d.contains("probe.asm(8)"), "{d}");
    // The ruling in one assertion: no number of attempts is ever shown.
    assert!(!d.contains("16"), "a pass count leaked into the message: {d}");
    assert!(!d.contains("pass"), "a pass count leaked into the message: {d}");
}

/// A reservation is sized the same way a repeat is, so the same loop is
/// available to it and the same proof closes it. `ds` reaches the old 16-pass
/// message too, which is why the row's own name (`AS-REPT-...`) understated the
/// problem.
///
/// asl, exit 2: `d1.asm(4): error: expression must be evaluatable in first pass`.
#[test]
fn refuses_a_reservation_sized_by_its_own_span() {
    let src = format!("{HEAD}Start:\n\tds.b N\nEnd:\nN\tequ End-Start+1\n\tdc.b $FF\n");
    let d = refusal(&src);
    assert!(d.contains("circular reservation size"), "{d}");
    assert!(d.contains("`ds.b`"), "{d}");
    assert!(d.contains("`N`"), "{d}");
    assert!(!d.contains("16"), "{d}");
}

/// Conjunct (a) does not need an equate in between: a repeat counting directly
/// by a label defined after it is the loop with one link instead of two.
///
/// asl, exit 2, same message.
#[test]
fn refuses_a_count_naming_a_forward_label_directly() {
    let src = format!("{HEAD}\trept End\n\tdc.b 0\n\tendr\nEnd:\n\tdc.b $FF\n");
    let d = refusal(&src);
    assert!(d.contains("circular repeat count"), "{d}");
    assert!(d.contains("`End`"), "{d}");
}

// ---------------------------------------------------------------------------
// DOES NOT FIRE WHEN IT SHOULD NOT
//
// These are the tests that matter. A false positive here refuses a program that
// works, which the ruling calls out as worse than the status quo it replaces.
// ---------------------------------------------------------------------------

/// Both labels sit BEHIND the repeat, so the count is known where the repeat
/// stands and conjunct (b) fails. Nothing about it is circular.
///
/// asl, exit 0, listing:
///
/// ```text
///        4/       0 : 0102 03             	dc.b 1,2,3
///        7/       3 : 00                  	dc.b 0
///        7/       4 : 00                  	dc.b 0
///        7/       5 : 00                  	dc.b 0
///        9/       6 : FF                  	dc.b $FF
/// ```
#[test]
fn accepts_backward_count() {
    let src =
        format!("{HEAD}Start:\n\tdc.b 1,2,3\nBack:\n\trept Back-Start\n\tdc.b 0\n\tendr\n\tdc.b $FF\n");
    assert_eq!(bytes(&src), vec![0x01, 0x02, 0x03, 0x00, 0x00, 0x00, 0xFF]);
}

/// **The first of the two false-positive tests.** The count names a symbol that
/// is not defined until after the repeat, so conjuncts (a) and (b) both hold and
/// a detector that stopped there would refuse this. It is not circular: `N` is a
/// plain constant, its value does not move when the repeat's size moves, and the
/// program settles in two passes.
///
/// Conjunct (d) is the whole reason this stays accepted.
///
/// asl REFUSES this (`c4.asm(3):7: error: expression must be evaluatable in
/// first pass`, exit 2) and supplies no bytes, so the expectation is sigil's
/// own, pinned. The ruling puts this shape in "everything else settles with no
/// fixed pass count"; the standing divergence from asl is recorded in the note.
#[test]
fn accepts_forward_constant_count() {
    let src = format!("{HEAD}\trept N\n\tdc.b 0\n\tendr\nLater:\n\tdc.b $FF\nN\tequ 4\n");
    assert_eq!(bytes(&src), vec![0x00, 0x00, 0x00, 0x00, 0xFF]);
}

/// **The second false-positive test, and the harder one.** Every link of the
/// loop is present EXCEPT the address flow: the count names `N`, `N` is defined
/// after the repeat, and `N` is derived from labels. But an `org` sits between
/// the repeat and `End`, so `End`'s address is not the repeat's address plus the
/// bytes the repeat emits, and the dependence is cut. The program settles.
///
/// A cycle detector built on the dependency graph alone refuses this, and it
/// would be wrong. `flow_epoch` is what sees the cut.
///
/// asl REFUSES (`c5.asm(4):7`, exit 2), so the expectation is sigil's own: the
/// repeat emits `$100` zero bytes and the trailing `$FF` lands at `$100`.
#[test]
fn accepts_count_separated_by_an_org() {
    let src = format!(
        "{HEAD}Start:\n\trept N\n\tdc.b 0\n\tendr\n\torg $100\nEnd:\nN\tequ End-Start\n\tdc.b $FF\n"
    );
    let out = bytes(&src);
    assert_eq!(out.len(), 0x101, "expected $100 zeros then the $FF");
    assert!(out[..0x100].iter().all(|b| *b == 0));
    assert_eq!(out[0x100], 0xFF);
}

/// A forward reference in ORDINARY data is not a layout expression at all: the
/// `dc.b` reserves one byte whatever `F` turns out to be, so no size depends on
/// it. Nothing here is watched, and this is the shape that keeps the rule from
/// spreading to every forward reference in a real disassembly.
///
/// asl, exit 0, listing `3/ 0 : 04    dc.b F`.
#[test]
fn accepts_forward_constant_data() {
    let src = format!("{HEAD}\tdc.b F\nF\tequ 4\n");
    assert_eq!(bytes(&src), vec![0x04]);
}

/// A location-derived equate BELOW a repeat with a literal count. Conjunct (a)
/// fails: the count never named `N`, so binding `N` from labels resolves no
/// watch. This is the ordinary "measure the block I just wrote" idiom and it
/// must stay ordinary.
///
/// asl, exit 0, listing:
///
/// ```text
///        5/       0 : 00                  	dc.b 0
///        5/       1 : 00                  	dc.b 0
///        5/       2 : 00                  	dc.b 0
///        8/       3 : =$3                  N	equ End-Start
///        9/       3 : 03                  	dc.b N
/// ```
#[test]
fn accepts_literal_count_with_a_derived_equ_below() {
    let src = format!("{HEAD}Start:\n\trept 3\n\tdc.b 0\n\tendr\nEnd:\nN\tequ End-Start\n\tdc.b N\n");
    assert_eq!(bytes(&src), vec![0x00, 0x00, 0x00, 0x03]);
}

/// An `if` condition decides layout and is deliberately NOT watched. This one
/// settles: the condition is false on the pass that seeded it and stays false,
/// so the guarded arm never emits and `End` never moves.
///
/// asl REFUSES (`d2.asm(4)`, exit 2), so the expectation is sigil's own.
#[test]
fn accepts_settling_if_condition() {
    let src = format!("{HEAD}Start:\n\tif End-Start>0\n\tdc.b 1,2,3\n\tendif\nEnd:\n\tdc.b $FF\n");
    assert_eq!(bytes(&src), vec![0xFF]);
}

// ---------------------------------------------------------------------------
// THE OSCILLATION REPORT
// ---------------------------------------------------------------------------

/// A genuine two-state oscillation, and the reason `if` is answered by the
/// oscillation proof rather than by the circularity proof. The condition is true
/// while the block is empty, which fills the block, which makes it false, which
/// empties it. There is no value for `End` and there never will be.
///
/// The proof needs no bound: the pass function is deterministic and its only
/// varying input is the previous environment, so an environment seen two or more
/// passes back repeats forever. What the author is told is what is moving and
/// between which values, which is what the ruling asked for, and NOT how many
/// times the assembler tried.
///
/// asl REFUSES (`d3.asm(4)`, exit 2), which is a different answer to the same
/// program and a correct one; sigil's is the ruling's answer.
#[test]
fn oscillation_names_the_moving_symbol_and_both_its_values() {
    let src = format!("{HEAD}Start:\n\tif End-Start=0\n\tdc.b 1,2,3\n\tendif\nEnd:\n\tdc.b $FF\n");
    let d = refusal(&src);
    assert!(d.contains("never settles"), "{d}");
    assert!(d.contains("`End`"), "{d}");
    // Both ends of the cycle, so the author can see the shape of it.
    assert!(d.contains('0') && d.contains('3'), "{d}");
    // The ruling, asserted directly: no count of attempts, in any wording.
    assert!(!d.contains("16"), "a pass count leaked into the message: {d}");
    assert!(!d.contains("pass"), "a pass count leaked into the message: {d}");
    assert!(!d.contains("converge"), "the old wording survived: {d}");
}
