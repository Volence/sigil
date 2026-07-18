//! Contract-grammar v2 §5 — verified `preserves` by symbolic stack tracking.
//!
//! The dataflow upgrade over the D2.32 syntactic movem-pair slice: `preserves(rN)`
//! holds iff on EVERY return path rN's value at `rts` equals its entry value —
//! restored from the matching stack slot, or never written. Handles individual
//! push/pop (branch-straddling), the `(sp)` peek, mid-body movem around a callee
//! call, and the movem entry/exit pair as the trivial fast path. Soundness
//! bailouts (computed sp, sp escaping into address math, displaced-sp stores that
//! could alias a tracked slot) mark the path UNVERIFIABLE.
//!
//! Every case is driven end-to-end from real `.emp` through `eval_proc_body`, so
//! the real `movem` RegList lowering + PreDec/PostInc operands are exercised, not
//! a hand-built stub. The shapes mirror the six G1-residue procs (AllocDynamic,
//! Collected_Park/UnparkSlot, Collected_CheckRing, Killed_CheckObject) — the real
//! procs are additionally covered by the corpus checkpoint.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::preserves::{verify_preserved, PreserveStatus};
use sigil_frontend_emp::value::Reg;
use sigil_ir::backend::Cpu;

/// Eval the first proc in `src` and return the preserve status of `reg`.
fn status(src: &str, reg: Reg) -> PreserveStatus {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "parse: {diags:?}");
    let p = file
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) => Some(p),
            _ => None,
        })
        .expect("a proc");
    let (buf, _d, _n) =
        eval_proc_body(&file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[]);
    let buf = buf.expect("codebuf");
    let mut r = verify_preserved(&buf.items, &[reg]);
    r.remove(&reg).expect("status for the checked reg")
}

fn is_verified(s: &PreserveStatus) -> bool {
    matches!(s, PreserveStatus::Verified)
}
fn is_unverifiable(s: &PreserveStatus) -> bool {
    matches!(s, PreserveStatus::Unverifiable(_))
}

// --- the "never written" clause -------------------------------------------

/// A proc that never writes a0 preserves it trivially — no stack machinery
/// needed (the `.full` return path of AllocDynamic).
#[test]
fn never_written_is_preserved() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             moveq #1, d0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "a0 never written → Verified, got {s:?}");
}

// --- individual push / pop (AllocDynamic's a0) -----------------------------

/// `move.l a0,-(sp)` … clobber a0 … `movea.l (sp)+,a0` … `rts`: a0 restored to
/// its entry value on the return path → Verified. The individual-push class
/// row 1030 could not express before §5.
#[test]
fn individual_push_pop_preserves() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             lea     Somewhere, a0\n\
             moveq   #0, d0\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "push/clobber/pop → Verified, got {s:?}");
}

/// A register saved, clobbered, but NOT restored before `rts` is genuinely
/// destroyed — NOT preserved (a declared `preserves` here would be a false
/// contract).
#[test]
fn written_without_restore_not_preserved() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             lea     Somewhere, a0\n\
             addq.l  #4, sp\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    // a0 was lea'd and the save was discarded via `addq #4,sp` (a computed-sp
    // pop we cannot model as a register restore) → not provably preserved.
    assert!(!is_verified(&s), "clobbered, not restored → not Verified, got {s:?}");
}

/// AllocDynamic's real shape: two independent push/pop return paths plus a third
/// path that never writes a0. Every return path preserves a0 → Verified.
#[test]
fn multi_return_paths_all_preserve() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             cmpi.w  #0, Flag\n\
             beq     .full\n\
             cmpi.w  #1, Other\n\
             bne     .append\n\
             move.l  a0, -(sp)\n\
             lea     A, a0\n\
             movea.l (sp)+, a0\n\
             moveq   #0, d0\n\
             rts\n\
         .append:\n\
             move.l  a0, -(sp)\n\
             lea     B, a0\n\
             movea.l (sp)+, a0\n\
             moveq   #0, d0\n\
             rts\n\
         .full:\n\
             moveq   #1, d0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "all three return paths preserve a0 → Verified, got {s:?}");
}

// --- the (sp) peek (Collected_Park/UnparkSlot's a0) ------------------------

/// The park/unpark shape: `move.l a0,-(sp)`; clobber a0; `movea.l (sp),a0`
/// (PEEK — restore from top without popping); clobber a0 again; `movea.l (sp)+,a0`
/// (final pop). a0 == entry at `rts` → Verified.
#[test]
fn peek_then_final_pop_preserves() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0, a1) {\n\
             move.l  a0, -(sp)\n\
             lea     A, a0\n\
             movea.l (sp), a0\n\
             lea     B, a0\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "peek + final pop → Verified, got {s:?}");
}

// --- mid-body movem around a call (Collected_CheckRing / Killed's d1) -------

/// `movem.l d0-d1,-(sp)`; `jbsr C` (clobbers d1); `movem.l (sp)+,d0-d1`: the
/// call trashes d1 but the matched movem pair restores it → Verified. This is
/// the mid-body movem class the D2.32 entry/exit slice cannot verify.
#[test]
fn midbody_movem_around_call_preserves() {
    let s = status(
        "module m\n\
         proc P () clobbers(d2, a0) {\n\
             movem.l d0-d1, -(sp)\n\
             jbsr    C\n\
             movem.l (sp)+, d0-d1\n\
             beq     .zero\n\
             move.w  d1, d2\n\
             rts\n\
         .zero:\n\
             moveq   #0, d2\n\
             rts\n\
         }\n",
        Reg::D1,
    );
    assert!(is_verified(&s), "movem save/restore around call → Verified, got {s:?}");
}

/// SOUNDNESS: a call must be treated as clobbering the checked register. Save d1,
/// call C, do NOT restore, return → NOT preserved (the callee could have trashed
/// d1). Without conservative call handling this would wrongly verify.
#[test]
fn call_without_restore_not_preserved() {
    let s = status(
        "module m\n\
         proc P () clobbers(d2, a0) {\n\
             movem.l d0-d1, -(sp)\n\
             jbsr    C\n\
             addq.l  #8, sp\n\
             rts\n\
         }\n",
        Reg::D1,
    );
    assert!(!is_verified(&s), "call, no restore → not Verified, got {s:?}");
}

/// The trivial fast path (HBlank_Dispatch / sound_api shape): a movem pair
/// wrapping the WHOLE body (entry save, exit restore) still verifies under the
/// general analysis — D2.32 subsumed, its adopters keep verifying.
#[test]
fn entry_exit_movem_fast_path_preserves() {
    let s = status(
        "module m\n\
         proc P () clobbers() {\n\
             movem.l d0-d2, -(sp)\n\
             moveq   #0, d0\n\
             moveq   #1, d1\n\
             moveq   #2, d2\n\
             movem.l (sp)+, d0-d2\n\
             rts\n\
         }\n",
        Reg::D1,
    );
    assert!(is_verified(&s), "entry/exit movem pair → Verified, got {s:?}");
}

/// A `dbf dN, .loop` copy loop between a save and restore (the Collected_Park/
/// UnparkSlot shape): a0 is pushed, advanced through the loop (`(a0)+`), then
/// popped back. The `dbcc` target is its SECOND operand — the CFG must resolve
/// it as a LOCAL back-edge, not an external `Defer`, or a0 falsely reports
/// NotPreserved.
#[test]
fn dbf_loop_between_save_and_restore_preserves() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0, d1, a1) {\n\
             move.l  a0, -(sp)\n\
             lea     A, a0\n\
             moveq   #3, d1\n\
         .loop:\n\
             move.b  (a0)+, (a1)+\n\
             dbf     d1, .loop\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "dbf-loop save/restore → Verified, got {s:?}");
}

// --- soundness bailouts ----------------------------------------------------

/// Computed sp: an `adda.w #n, sp` moves the stack pointer by an amount the slot
/// model cannot track → the path is UNVERIFIABLE (error-tier for a declared
/// preserves — a wrong contract is worse than none).
#[test]
fn computed_sp_is_unverifiable() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             lea     A, a0\n\
             adda.w  #4, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_unverifiable(&s), "computed sp → Unverifiable, got {s:?}");
}

/// sp escaping into an address register (`movea.l sp, a3`) means the saved
/// region can be reached through a non-sp base later — the slot model can no
/// longer prove non-aliasing → UNVERIFIABLE.
#[test]
fn sp_escape_to_areg_is_unverifiable() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0, a3) {\n\
             move.l  a0, -(sp)\n\
             movea.l sp, a3\n\
             lea     A, a0\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_unverifiable(&s), "sp escape → Unverifiable, got {s:?}");
}

/// A displaced-sp STORE (`move.l d0, 4(sp)`) writes into the saved region and
/// could alias a tracked slot → UNVERIFIABLE (the TrySpawnObject spill-frame
/// hazard the G1 notes flagged).
#[test]
fn displaced_sp_store_is_unverifiable() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             move.l  d0, 4(sp)\n\
             lea     A, a0\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_unverifiable(&s), "displaced-sp store → Unverifiable, got {s:?}");
}
