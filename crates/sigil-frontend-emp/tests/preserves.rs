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
use sigil_frontend_emp::closure::RegEffect;
use sigil_frontend_emp::preserves::{verify_preserved, CallPolicy, PreserveStatus};
use sigil_frontend_emp::value::Reg;
use sigil_ir::backend::Cpu;
use std::collections::{BTreeMap, BTreeSet};

/// Eval the first proc in `src` and return the preserve status of `reg` under
/// `policy`.
fn status_with(src: &str, reg: Reg, policy: CallPolicy) -> PreserveStatus {
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
        eval_proc_body(&file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[], &sigil_frontend_emp::contract::InterfaceEnv::empty());
    let buf = buf.expect("codebuf");
    let mut r = verify_preserved(&buf.items, &[reg], policy);
    r.remove(&reg).expect("status for the checked reg")
}

/// Eval + status under the conservative (no callee-contract) model — the byte
/// gate's default and the pre-oracle behavior.
fn status(src: &str, reg: Reg) -> PreserveStatus {
    status_with(src, reg, CallPolicy::ClobberAll)
}

/// A one-entry `effective` oracle map: `callee` clobbers exactly `clobbers`.
fn oracle(callee: &str, clobbers: &[&str]) -> BTreeMap<String, RegEffect> {
    let mut m = BTreeMap::new();
    m.insert(
        callee.to_string(),
        RegEffect { top: false, regs: clobbers.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>() },
    );
    m
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
    // a0 was lea'd and its save DISCARDED by the immediate cleanup `addq #4,sp`
    // (which drops the whole a0 slot, no restore) → genuinely not preserved.
    assert!(!is_verified(&s), "clobbered, save discarded → not Verified, got {s:?}");
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

/// REGRESSION PIN for the shared-CFG `dbcc dN, label` target resolution (the
/// checkpoint's finding). `dbcc`'s label is its SECOND operand; if the CFG reads
/// the FIRST (`ops.first()` — the counter register), the taken edge misresolves
/// to an external `Defer` and the loop-carried save falsely reports NotPreserved.
/// Here a0 is preserved ONLY because the `dbcc` back-edge stays local: the pop
/// after the loop restores it. A first-operand regression flips this to
/// NotPreserved. (`dbeq`/`dbcc`/`dbf` all share the two-operand shape.)
#[test]
fn dbcc_target_is_second_operand() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0, d1, a1) {\n\
             move.l  a0, -(sp)\n\
             lea     A, a0\n\
             moveq   #3, d1\n\
         .scan:\n\
             tst.b   (a0)+\n\
             dbeq    d1, .scan\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "dbcc second-operand target must resolve local, got {s:?}");
}

// --- soundness bailouts ----------------------------------------------------

/// A `movem.w` restore SIGN-EXTENDS each word into the full 32-bit register — it
/// does NOT preserve the register. A `.w` save/restore pair must NOT verify (the
/// tranche-3 word-pair finding, now subsumed by §5's size-awareness).
#[test]
fn movem_w_pair_does_not_preserve() {
    let s = status(
        "module m\n\
         proc P () clobbers() {\n\
             movem.w d0-d1, -(sp)\n\
             moveq   #5, d0\n\
             movem.w (sp)+, d0-d1\n\
             rts\n\
         }\n",
        Reg::D1,
    );
    assert!(!is_verified(&s), "movem.w pair must not verify (sign-extends), got {s:?}");
}

/// A pop that drains MORE than was pushed underflows the tracked stack — it reads
/// the caller's frame / return address, so the model is inconsistent →
/// Unverifiable. (The wrong-list early-exit pop the tranche-3 slice caught.)
#[test]
fn pop_underflow_is_unverifiable() {
    let s = status(
        "module m\n\
         proc P () clobbers() {\n\
             movem.l d0-d1, -(sp)\n\
             movem.l (sp)+, d0-d2\n\
             rts\n\
         }\n",
        Reg::D1,
    );
    assert!(is_unverifiable(&s), "pop underflow → Unverifiable, got {s:?}");
}

/// A soundness bailout on a NORETURN path (a `subq #2,sp` before a `jmp` to an
/// external error handler — the DEBUG `assert`/`raise_error` shape) must NOT
/// poison the verification of the RETURNING paths. a0 is push/pop-preserved on
/// the rts path; the bailed path diverges (never returns), so a0 still verifies.
/// (Path-LOCAL bailout, not global.)
#[test]
fn bailout_on_noreturn_path_does_not_poison_returns() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             lea     X, a0\n\
             tst.b   d0\n\
             bne     .raise\n\
             movea.l (sp)+, a0\n\
             rts\n\
         .raise:\n\
             subq.w  #2, sp\n\
             jmp     MDDBG__ErrorHandler\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "bail on the noreturn path must not poison a0, got {s:?}");
}

/// Companion: a bailout on a RETURNING path DOES make the register unverifiable
/// (the stack model is untrustworthy where it matters).
#[test]
fn bailout_on_returning_path_is_unverifiable() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             lea     X, a0\n\
             adda.w  #2, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_unverifiable(&s), "bail on the rts path → Unverifiable, got {s:?}");
}

/// Computed sp: an `adda.w d1, sp` (REGISTER source) moves the stack pointer by a
/// runtime amount the slot model cannot track → the path is UNVERIFIABLE
/// (error-tier for a declared preserves — a wrong contract is worse than none).
/// The immediate cleanup form is modeled separately (the sp-cleanup idiom); only a
/// COMPUTED (register/index) sp op stays a hard bailout here.
#[test]
fn computed_sp_is_unverifiable() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0, d1) {\n\
             move.l  a0, -(sp)\n\
             lea     A, a0\n\
             adda.w  d1, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_unverifiable(&s), "computed (register) sp → Unverifiable, got {s:?}");
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

// --- §5 linear-delta tracker (address-register arithmetic round-trips) --------
// preserves(rN) where rN is a POINTER advanced and restored by static arithmetic
// — the DeleteObject `.clear_slot` idiom (clear_longs advances (a0)+ N times, then
// `lea -sizeof(Sst)(a0), a0` restores). §5's stack model cannot see this (no save);
// the linear-delta tracker proves it: preserved iff the net Δ == 0 at every rts and
// the register is only ever modified by trackable arithmetic.

fn is_not_preserved(s: &PreserveStatus) -> bool {
    matches!(s, PreserveStatus::NotPreserved)
}

/// a0 advanced by two `(a0)+` (+4 each) then restored by `lea -8(a0), a0` — net
/// Δ == 0 → Verified (the DeleteObject shape, straight-line after comptime fold).
#[test]
fn pointer_advance_then_restore_verifies() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l d0, (a0)+\n\
             move.l d0, (a0)+\n\
             lea -8(a0), a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "advance +8 then lea -8 → Δ=0 → Verified, got {s:?}");
}

/// The restoring `lea` is MISSING — a0 ends +8 from entry → NotPreserved (a
/// genuinely-advanced pointer must not be falsely verified).
#[test]
fn pointer_advance_without_restore_not_preserved() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l d0, (a0)+\n\
             move.l d0, (a0)+\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_not_preserved(&s), "Δ=+8 at rts → NotPreserved, got {s:?}");
}

/// A RUNTIME loop advances a0 a trip-count-dependent number of times — the delta
/// is not statically known (the loop join reconciles Δ=0 with Δ=+4 → untrackable)
/// → must NOT be falsely verified.
#[test]
fn runtime_loop_advance_is_not_falsely_verified() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0-d1) {\n\
             moveq #3, d1\n\
         .loop:\n\
             move.l d0, (a0)+\n\
             dbf d1, .loop\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(!is_verified(&s), "runtime-variable Δ must not verify, got {s:?}");
}

/// adda/suba immediates accumulate too: `adda.w #6, a0` then `suba.w #6, a0` → Δ=0.
#[test]
fn adda_suba_immediates_round_trip_verifies() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             adda.w #6, a0\n\
             move.w d0, (a0)\n\
             suba.w #6, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "adda +6 then suba -6 → Δ=0 → Verified, got {s:?}");
}

/// A FRESH load of the register (`movea.l`) makes the delta untrackable — even if
/// followed by arithmetic that nets zero, the value is no longer entry+Δ.
#[test]
fn fresh_load_makes_delta_untrackable() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0/a1) {\n\
             movea.l a1, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(!is_verified(&s), "a0 loaded from a1 → not its entry value, got {s:?}");
}

// --- §5 grow: displaced-sp READ is safe (the movem-frame-slot recovery idiom) --
//
// `sp_hazard` bailed on ANY `d(sp)` operand, but its own rationale ("could alias
// a saved slot") is true only for a WRITE. A displaced-sp READ into a plain
// register (`movea.l 8(sp), aN` — the frame-slot recovery in TrySpawnObject /
// TrySpawnRing) cannot alter a tracked slot; it is the displaced analogue of the
// `(sp)` peek and takes the normal register-write handling. The SAFE-LIST is
// EXACTLY move/movea with a DispInd{a7} SOURCE and a plain non-a7 register dest;
// everything else keeps bailing.

/// POSITIVE: a movem save/restore of d3 with a displaced-sp frame READ in between
/// (`movea.l 4(sp), a0`) now VERIFIES d3 preserved — the read no longer poisons
/// the proof. (Mirrors EntityWindow_TrySpawnObject's `movea.l 8(sp), a0`.)
#[test]
fn displaced_sp_read_into_reg_is_safe() {
    let s = status(
        "module m\n\
         proc P () clobbers(a0) {\n\
             movem.l d3/a0, -(sp)\n\
             movea.l 4(sp), a0\n\
             moveq #5, d3\n\
             movem.l (sp)+, d3/a0\n\
             rts\n\
         }\n",
        Reg::D3,
    );
    assert!(is_verified(&s), "movem save/restore around a displaced-sp READ → d3 Verified, got {s:?}");
}

/// NEGATIVE (store): a displaced-sp WRITE target (`move.l d0, 8(sp)`) could alias
/// a saved slot — keeps bailing.
#[test]
fn displaced_sp_write_target_still_bails() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             moveq #5, d3\n\
             move.l d0, 8(sp)\n\
             rts\n\
         }\n",
        Reg::D3,
    );
    assert!(is_unverifiable(&s), "displaced-sp STORE aliases the slot model → Unverifiable, got {s:?}");
}

/// NEGATIVE (rmw, read-direction): `add.l 8(sp), d0` is not move/movea — the
/// safe-list is deliberately move/movea only (minimality over cleverness).
#[test]
fn displaced_sp_rmw_still_bails() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             moveq #5, d3\n\
             add.l 8(sp), d0\n\
             rts\n\
         }\n",
        Reg::D3,
    );
    assert!(is_unverifiable(&s), "add.l d(sp),dN is not on the move/movea safe-list → Unverifiable, got {s:?}");
}

/// NEGATIVE (address escape): `pea 8(sp)` computes an effective address, not a
/// data read — the sp value escapes. Keeps bailing.
#[test]
fn pea_displaced_sp_still_bails() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             moveq #5, d3\n\
             pea 8(sp)\n\
             rts\n\
         }\n",
        Reg::D3,
    );
    assert!(is_unverifiable(&s), "pea d(sp) escapes the sp value → Unverifiable, got {s:?}");
}

/// NEGATIVE (dest is sp): `movea.l 8(sp), sp` is stack replacement, not a
/// frame-slot recovery — a non-register (a7) destination keeps bailing.
#[test]
fn displaced_sp_read_into_sp_still_bails() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             moveq #5, d3\n\
             movea.l 8(sp), sp\n\
             rts\n\
         }\n",
        Reg::D3,
    );
    assert!(is_unverifiable(&s), "movea.l d(sp),sp replaces the stack pointer → Unverifiable, got {s:?}");
}

// ===========================================================================
// §5 — the CALLEE-PRESERVES ORACLE (t30). A `preserves(rN)` that round-trips
// rN then makes a TRAILING call to a callee that itself `preserves(rN)` is not
// provable by the conservative model (the call clobbers all), but IS provable
// once the callee's verified contract is credited. The `TestChurnObj_Main`
// shape: pop a0, `jsr DeleteObject` (preserves a0), rts.
// ===========================================================================

/// The reproducing fixture: a0 saved, clobbered, restored, then a TRAILING call
/// runs before `rts`. Under the conservative model the trailing call clobbers
/// a0 → NotPreserved.
const TRAILING_CALL_SHAPE: &str = "module m\n\
     proc P () clobbers(d0) {\n\
         move.l  a0, -(sp)\n\
         lea     Foo, a0\n\
         movea.l (sp)+, a0\n\
         jsr     DeleteObject\n\
         rts\n\
     }\n";

#[test]
fn trailing_call_conservative_is_not_preserved() {
    // ClobberAll: the trailing call is treated as clobbering a0 → NotPreserved.
    // This is the demand the oracle closes.
    let s = status_with(TRAILING_CALL_SHAPE, Reg::A0, CallPolicy::ClobberAll);
    assert!(
        matches!(s, PreserveStatus::NotPreserved),
        "conservative model: trailing call clobbers a0 → NotPreserved, got {s:?}"
    );
}

#[test]
fn trailing_call_oracle_preserving_callee_verifies() {
    // Oracle where DeleteObject preserves a0 (clobbers only d0-d1) → the call no
    // longer clobbers a0 → Verified. THE feature.
    let eff = oracle("DeleteObject", &["d0", "d1"]);
    let s = status_with(TRAILING_CALL_SHAPE, Reg::A0, CallPolicy::Oracle(&eff));
    assert!(
        matches!(s, PreserveStatus::Verified),
        "oracle: a preserving callee lets a0 round-trip → Verified, got {s:?}"
    );
}

#[test]
fn trailing_call_optimistic_probe_verifies() {
    // PreserveAll (the byte gate's DEFER probe): every call preserves → a0
    // verifies, so the byte gate knows the ONLY obstacle is the call and DEFERS.
    let s = status_with(TRAILING_CALL_SHAPE, Reg::A0, CallPolicy::PreserveAll);
    assert!(
        matches!(s, PreserveStatus::Verified),
        "optimistic probe: call-only obstacle → Verified (defer), got {s:?}"
    );
}

// --- t24 controls ----------------------------------------------------------

#[test]
fn t24_callee_without_preserves_still_clobbers() {
    // Control: a callee whose effective set INCLUDES a0 (it does NOT preserve it)
    // still clobbers a0 under the oracle → NotPreserved. The oracle credits only
    // a genuinely-preserving callee.
    let eff = oracle("DeleteObject", &["a0", "d0"]);
    let s = status_with(TRAILING_CALL_SHAPE, Reg::A0, CallPolicy::Oracle(&eff));
    assert!(
        matches!(s, PreserveStatus::NotPreserved),
        "callee that clobbers a0 → NotPreserved even under the oracle, got {s:?}"
    );
}

#[test]
fn t24_unknown_or_cycle_callee_stays_conservative() {
    // Control: a callee ABSENT from the effective map (an unresolved target, or an
    // in-cycle member the closure could not credit) reads as clobber-all → the
    // trailing call clobbers a0 → NotPreserved. Cycles need no fixpoint here.
    let empty: BTreeMap<String, RegEffect> = BTreeMap::new();
    let s = status_with(TRAILING_CALL_SHAPE, Reg::A0, CallPolicy::Oracle(&empty));
    assert!(
        matches!(s, PreserveStatus::NotPreserved),
        "unknown/cycle callee stays conservative → NotPreserved, got {s:?}"
    );
}

#[test]
fn t24_top_effect_callee_clobbers() {
    // Control: a callee with a ⊤ (unbounded) effect clobbers everything → a0 not
    // credited even though its concrete `regs` set is empty.
    let mut eff: BTreeMap<String, RegEffect> = BTreeMap::new();
    eff.insert("DeleteObject".to_string(), RegEffect { top: true, regs: Default::default() });
    let s = status_with(TRAILING_CALL_SHAPE, Reg::A0, CallPolicy::Oracle(&eff));
    assert!(
        matches!(s, PreserveStatus::NotPreserved),
        "⊤-effect callee clobbers a0 → NotPreserved, got {s:?}"
    );
}

#[test]
fn t24_genuine_local_clobber_is_not_deferrable() {
    // Control: a clobber AFTER the trailing preserving call with NO restore is a
    // GENUINE local failure the oracle cannot fix — it fails even the optimistic
    // probe, so the byte gate must NOT defer it (it stays a real error).
    let src = "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             movea.l (sp)+, a0\n\
             jsr     DeleteObject\n\
             lea     Foo, a0\n\
             rts\n\
         }\n";
    // Optimistic (every call preserves) STILL fails — the trailing lea clobbers a0
    // with no restore.
    let opt = status_with(src, Reg::A0, CallPolicy::PreserveAll);
    assert!(
        matches!(opt, PreserveStatus::NotPreserved),
        "a post-call clobber is a genuine failure, not deferrable → NotPreserved, got {opt:?}"
    );
    // The oracle likewise cannot save it.
    let eff = oracle("DeleteObject", &["d0"]);
    let s = status_with(src, Reg::A0, CallPolicy::Oracle(&eff));
    assert!(
        matches!(s, PreserveStatus::NotPreserved),
        "oracle cannot fix a genuine local clobber → NotPreserved, got {s:?}"
    );
}

#[test]
fn t24_sp_hazard_is_not_deferrable() {
    // Control: an sp bailout is a local hazard no callee contract can fix — it
    // stays Unverifiable under the optimistic probe (not deferrable).
    let src = "module m\n\
         proc P () clobbers(d0) {\n\
             moveq   #5, d3\n\
             pea     4(sp)\n\
             jsr     DeleteObject\n\
             rts\n\
         }\n";
    let opt = status_with(src, Reg::D3, CallPolicy::PreserveAll);
    assert!(
        matches!(opt, PreserveStatus::Unverifiable(_)),
        "an sp hazard is not deferrable → Unverifiable even optimistically, got {opt:?}"
    );
}

// --- the immediate-sp-cleanup idiom (t38 player_sensors probe cores) ----------
// `move.l a0,-(sp)` … push scratch … drop the scratch with `addq/adda #N,sp` …
// `movea.l (sp)+,a0`: the probe cores save a0 across Collision_GetType, stash the
// layer word beneath it, then pop the layer with an immediate sp-INCREASE before
// restoring a0. The old model bailed on ALL explicit sp arithmetic; §5 now models
// an IMMEDIATE sp-INCREASE as dropping exactly N bytes of tracked slots, so the
// round-trip verifies. Slots carry their TRUE pushed byte width (move.w=2 vs
// move.l=4 vs per-movem-member) — the prerequisite for a whole-slot drop.

/// C1 — the core positive: an immediate `addq #2,sp` drops the WORD scratch slot
/// exactly, leaving a0's long slot for the final pop → Verified.
#[test]
fn sp_cleanup_immediate_increase_over_word_slot_preserves() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             move.w  d3, -(sp)\n\
             lea      A, a0\n\
             addq.l  #2, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "addq #2 drops the word slot, a0 pops from its long slot → Verified, got {s:?}");
}

/// C1 control — a constant sp-DECREASE (`subq #4,sp`, a scratch alloc) is NOT the
/// cleanup idiom and stays a soundness bailout → Unverifiable.
#[test]
fn sp_cleanup_constant_decrease_stays_bailing() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             lea      A, a0\n\
             subq.l  #4, sp\n\
             addq.l  #4, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_unverifiable(&s), "sp-DECREASE is not the cleanup idiom → Unverifiable, got {s:?}");
}

/// C1 control — a COMPUTED sp-increase (register source, `adda d1,sp`) is not an
/// immediate cleanup; the drop amount is unknown → stays bailing.
#[test]
fn sp_cleanup_computed_increase_register_stays_bailing() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0, d1) {\n\
             move.l  a0, -(sp)\n\
             lea      A, a0\n\
             adda.l  d1, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_unverifiable(&s), "computed (register) sp-increase → Unverifiable, got {s:?}");
}

/// C2 — the slot-boundary rule: a drop that consumes only PART of a slot (`addq
/// #2,sp` over a 4-byte long slot) cannot be modeled and bails loudly.
#[test]
fn sp_cleanup_partial_slot_drop_bails() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             lea      A, a0\n\
             addq.l  #2, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_unverifiable(&s), "a partial-slot drop (2 of a 4-byte slot) must bail → Unverifiable, got {s:?}");
}

/// C3 — over-drop soundness: a cleanup that consumes a0's OWN saved slot (dropping
/// the wrong 4 bytes) leaves a decoy slot below; the later pop reads the decoy, so
/// a0 ends != entry → preserves is REFUTED (NotPreserved, not merely unverifiable).
#[test]
fn sp_cleanup_over_drop_reads_wrong_slot_refutes() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a1, -(sp)\n\
             move.l  a0, -(sp)\n\
             lea      A, a0\n\
             addq.l  #4, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_not_preserved(&s), "over-drop consumes a0's slot, pop reads a1's decoy → NotPreserved, got {s:?}");
}

/// C4 — a `move.w` push is tracked as a TRUE 2-byte slot: `addq #2,sp` lands on
/// its boundary exactly and verifies. (If the word slot were mis-sized as 4, this
/// same drop would be a partial-slot bail — the width representation is the proof.)
#[test]
fn sp_cleanup_word_slot_width_is_two_bytes() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0) {\n\
             move.l  a0, -(sp)\n\
             move.w  d1, -(sp)\n\
             lea      A, a0\n\
             addq.l  #2, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "a move.w slot is 2 bytes, dropped exactly by addq #2 → Verified, got {s:?}");
}

/// C4 — movem member widths are TRUE per-member sizes: three `movem.w` members are
/// 2 bytes each (6 total), dropped exactly by `addq #6,sp` → Verified.
#[test]
fn sp_cleanup_movem_word_members_are_two_bytes_each() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0-d2) {\n\
             move.l  a0, -(sp)\n\
             movem.w d0-d2, -(sp)\n\
             lea      A, a0\n\
             addq.l  #6, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "three movem.w members = 6 bytes, dropped exactly → Verified, got {s:?}");
}

/// C5 — the real probe-core shape end to end: save a0, stash the layer word, call
/// a clobbering callee, drop the layer with `addq #2,sp`, restore a0. Under the
/// conservative ClobberAll model a0 verifies (the save round-trips regardless of
/// what the callee did).
#[test]
fn sp_cleanup_probe_core_shape_with_call_preserves_a0() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0-d3, a1) {\n\
             move.l  a0, -(sp)\n\
             move.w  d3, -(sp)\n\
             jbsr    Collision_GetType\n\
             addq.l  #2, sp\n\
             movea.l (sp)+, a0\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(is_verified(&s), "probe-core save/cleanup/restore → Verified even under ClobberAll, got {s:?}");
}

/// C5 non-vacuity — the SAME shape with the final restore REMOVED must not verify
/// (the layer drop discarded a0's save and nothing restores it) → an injected
/// false preserves fires.
#[test]
fn sp_cleanup_probe_core_missing_restore_does_not_preserve() {
    let s = status(
        "module m\n\
         proc P () clobbers(d0-d3, a1) {\n\
             move.l  a0, -(sp)\n\
             move.w  d3, -(sp)\n\
             jbsr    Collision_GetType\n\
             addq.l  #6, sp\n\
             rts\n\
         }\n",
        Reg::A0,
    );
    assert!(!is_verified(&s), "no restore → a0 not preserved, got {s:?}");
}
