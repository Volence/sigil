//! `[branch.condition-constant]` — the sound "a conditional branch whose reaching
//! CCR-definition is a compile-time constant" check (item-4 rider, §D backlog).
//!
//! Each test pins one rule of the sound formulation (no intent inference — fire
//! ONLY when CCR is provably constant on every reaching path):
//! - the `Sound_PlayMusic.await_slot` bug shape (a constant-immediate write
//!   between a value test and the branch) FIRES;
//! - the legitimate `btst`/`cmp`/`tst` spins do NOT fire (their CCR is runtime);
//! - a `moveq`-fed branch fires (constant, the generic dead-branch class);
//! - meet-disagreeing constants across a join do NOT fire (the outcome is not
//!   statically determined).

use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::parse_str;

fn analyze(src: &str) -> ContractReport {
    let (f, diags) = parse_str(src);
    assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
    analyze_corpus(&[f])
}

fn bc_count(r: &ContractReport, proc: &str) -> usize {
    r.branch_const_firings.iter().filter(|f| f.proc == proc).count()
}

/// The `await_slot` shape: a constant-immediate write (the bus release,
/// `move.w #$0000, …`) sits between the `tst` and the `bne`, so the `bne` reads
/// the move's forced `Z`, never the `tst`. The back-edge is statically dead.
#[test]
fn constant_flag_write_before_branch_fires() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers(d0/d1) {\n\
         .spin:\n\
             tst.b   d0\n\
             move.w  #$0000, d1\n\
             bne     .spin\n\
             rts\n\
         }\n",
    );
    assert_eq!(bc_count(&r, "P"), 1, "{:?}", r.branch_const_firings);
}

/// The legitimate `stopZ80` inner spin: `btst` sets `Z` from a runtime bus-grant
/// bit, so the `bne` condition is NOT constant. Must NOT fire.
#[test]
fn btst_spin_does_not_fire() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers(d0) {\n\
         .wait:\n\
             btst    #0, d0\n\
             bne     .wait\n\
             rts\n\
         }\n",
    );
    assert_eq!(bc_count(&r, "P"), 0, "{:?}", r.branch_const_firings);
}

/// A normal compare-and-branch: `cmpi` sets CCR from a runtime register. No fire.
#[test]
fn cmp_branch_does_not_fire() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers(d0) {\n\
             cmpi.w  #5, d0\n\
             bne     .x\n\
         .x:\n\
             rts\n\
         }\n",
    );
    assert_eq!(bc_count(&r, "P"), 0, "{:?}", r.branch_const_firings);
}

/// A `moveq`-fed branch: the reaching CCR-def is a constant immediate, so the
/// `beq` outcome is statically determined (dead fall-through). Fires.
#[test]
fn moveq_fed_branch_fires() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers(d0) {\n\
             moveq   #0, d0\n\
             beq     .x\n\
         .x:\n\
             rts\n\
         }\n",
    );
    assert_eq!(bc_count(&r, "P"), 1, "{:?}", r.branch_const_firings);
}

/// Two paths reach the guard with DIFFERENT constant flags (`moveq #1` clears Z,
/// `moveq #0` sets Z), so the meet is `Dyn` and the outcome is not statically
/// determined. Must NOT fire (soundness of the join = meet).
#[test]
fn meet_disagreeing_constants_does_not_fire() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers(d0/d1) {\n\
             tst.b   d0\n\
             beq     .zero\n\
             moveq   #1, d1\n\
             bra     .join\n\
         .zero:\n\
             moveq   #0, d1\n\
         .join:\n\
             beq     .out\n\
         .out:\n\
             rts\n\
         }\n",
    );
    assert_eq!(bc_count(&r, "P"), 0, "{:?}", r.branch_const_firings);
}
