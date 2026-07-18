//! Contract-grammar v2 §6 — the caller-side flag-result must-use check
//! ([call.flag-result-unused]). Exercises the lightweight CFG over an evaluated
//! CodeBuf (real joins, not straight-line) + the carry consume/redefine tables,
//! end-to-end from `.emp` source through `eval_proc_body` — so the dplc
//! `movem.l (sp)+` transparency (the code's own hazard note) is covered against
//! the real evaluator, not a hand-built stub.

use sigil_frontend_emp::ast::{AsmStmt, Item};
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::flag_check::{
    check_flag_unused, check_result_invalid_path, FlagFiring, FlagFiringKind,
};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet};

/// Eval the first proc in `src` and run the flag-unused check, with `callee`
/// declared to return carry and `discarded` the set of call-site spans opted
/// out. Returns the firings.
fn run(src: &str, callee: &str, discarded: &[Span]) -> Vec<FlagFiring> {
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
    let mut fc: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    fc.insert(callee.to_string(), BTreeSet::from(["carry".to_string()]));
    check_flag_unused(&p.name, &buf.items, &fc, discarded)
}

/// The AST spans of instructions carrying `@discards`, for the opt-out set.
fn discards_spans(src: &str) -> Vec<Span> {
    let (file, _d) = parse_str(src);
    let mut out = Vec::new();
    for item in &file.items {
        if let Item::Proc(p) = item {
            for s in &p.body {
                if let AsmStmt::Instr(i) = s {
                    if i.discards.is_some() {
                        out.push(i.span);
                    }
                }
            }
        }
    }
    out
}

const NONE: &[Span] = &[];

/// A dropped carry — the callee's flag is never read; the code overwrites CC
/// (a data op) and returns. This is the Palette_Dirty / load_art bug class.
#[test]
fn dropped_carry_fires() {
    let f = run(
        "module m\n\
         proc P () clobbers(d0-d4/a1-a2) {\n\
             jbsr Queue\n\
             moveq #0, d0\n\
             rts\n\
         }\n",
        "Queue",
        NONE,
    );
    assert_eq!(f.len(), 1, "expected one firing, got {f:?}");
    assert_eq!(f[0].callee, "Queue");
    assert_eq!(f[0].flag, "carry");
}

/// A consumed carry — `bcs` reads it before any redefine. No firing.
#[test]
fn consumed_carry_passes() {
    let f = run(
        "module m\n\
         proc P () clobbers(d0-d4/a1-a2) {\n\
             jbsr Queue\n\
             bcs .done\n\
             moveq #0, d0\n\
         .done:\n\
             rts\n\
         }\n",
        "Queue",
        NONE,
    );
    assert!(f.is_empty(), "carry consumed by bcs — should not fire: {f:?}");
}

/// The dplc pattern: a `movem.l (sp)+` sits between the call and its `bcs`.
/// `movem` preserves CCR, so the carry survives — must NOT fire (the code's own
/// hazard note; a naive "any instruction redefines CC" model would false-fire).
#[test]
fn movem_between_call_and_bcs_is_transparent() {
    let f = run(
        "module m\n\
         proc P () clobbers(d0-d4/a1-a2) {\n\
             jbsr Queue\n\
             movem.l (sp)+, d2-d4/a2-a3\n\
             bcs .done\n\
             moveq #0, d0\n\
         .done:\n\
             rts\n\
         }\n",
        "Queue",
        NONE,
    );
    assert!(f.is_empty(), "movem is CC-transparent — carry survives to bcs: {f:?}");
}

/// An `addx` between the call and a `bcs` REDEFINES carry (it reads X, not the
/// callee's C, and writes a fresh C) — so the later `bcs` tests the wrong flag
/// and the callee's result is dropped. Must fire (Fable's G2.6 rider: the
/// ADDX-class is a carry WRITER, not a consumer, for a carry result).
#[test]
fn addx_between_call_and_bcs_redefines_and_fires() {
    let f = run(
        "module m\n\
         proc P () clobbers(d0-d1) {\n\
             jbsr Queue\n\
             addx.w d0, d1\n\
             bcs .done\n\
             moveq #0, d0\n\
         .done:\n\
             rts\n\
         }\n",
        "Queue",
        NONE,
    );
    assert_eq!(f.len(), 1, "addx redefines carry before the bcs — must fire: {f:?}");
}

/// A `move.w #imm, sr` between the call and a `bcs` writes the whole status
/// register — carry included — so the callee's result is lost. Must fire
/// (the move-to-ccr/sr redefine, Fable's rider).
#[test]
fn move_to_sr_between_call_and_bcs_redefines_and_fires() {
    let f = run(
        "module m\n\
         proc P () clobbers(d0) {\n\
             jbsr Queue\n\
             move.w #$2700, sr\n\
             bcs .done\n\
             moveq #0, d0\n\
         .done:\n\
             rts\n\
         }\n",
        "Queue",
        NONE,
    );
    assert_eq!(f.len(), 1, "move to sr redefines carry before the bcs — must fire: {f:?}");
}

/// A return without consuming the carry fires (the flag is abandoned in the
/// frame that must consume it).
#[test]
fn return_without_consume_fires() {
    let f = run(
        "module m\n\
         proc P () clobbers(d0) {\n\
             jbsr Queue\n\
             rts\n\
         }\n",
        "Queue",
        NONE,
    );
    assert_eq!(f.len(), 1, "return abandons the flag — should fire: {f:?}");
}

/// A branch join where ONE path consumes the carry and the other returns
/// unconsumed fires (must-use is every-path — this is why the CFG needs joins,
/// not straight-line).
#[test]
fn one_unconsumed_path_at_a_join_fires() {
    let f = run(
        "module m\n\
         proc P () clobbers(d0) {\n\
             jbsr Queue\n\
             bne .skip\n\
             bcs .done\n\
         .skip:\n\
             rts\n\
         .done:\n\
             rts\n\
         }\n",
        "Queue",
        NONE,
    );
    assert_eq!(f.len(), 1, "the .skip path returns unconsumed — should fire: {f:?}");
}

/// `@discards(dropped)` on the call is the explicit opt-out — the same dropped
/// carry that fires without it does NOT fire with it (AST span → CodeBuf span).
#[test]
fn discards_suppresses_the_firing() {
    let src = "module m\n\
               proc P () clobbers(d0) {\n\
                   jbsr Queue @discards(dropped)\n\
                   moveq #0, d0\n\
                   rts\n\
               }\n";
    let with_discard = run(src, "Queue", &discards_spans(src));
    assert!(with_discard.is_empty(), "@discards must suppress: {with_discard:?}");
    // ...and without the opt-out it DOES fire (proving the span is what matters).
    let without = run(src, "Queue", NONE);
    assert_eq!(without.len(), 1, "same call fires without the discard span");
}

// ---------------------------------------------------------------------------
// §6 / G2.4 — [call.result-invalid-path] for out(rN if cc) conditional register
// results. Reading rN on the path where cc says it is invalid fires. Forward
// machinery: no corpus site declares a conditional register result today.
// ---------------------------------------------------------------------------

/// Eval the first proc and run the invalid-path check, with `callee` declared to
/// return `reg` valid only when `cc` holds.
fn run_cond(src: &str, callee: &str, reg: &str, cc: &str) -> Vec<FlagFiring> {
    run_invalid(src, &[(callee, reg, cc)], &[])
}

/// General form: `cond` = conditional-out callees `(callee, reg, cc)`; `uncond` =
/// callees with UNCONDITIONAL outs `(callee, &[regs])` (which the shared
/// call-aware primitive treats as taint-killing redefines).
fn run_invalid(
    src: &str,
    cond: &[(&str, &str, &str)],
    uncond: &[(&str, &[&str])],
) -> Vec<FlagFiring> {
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
    let mut cc_callees: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (callee, reg, cc) in cond {
        cc_callees
            .entry(callee.to_string())
            .or_default()
            .push((reg.to_string(), cc.to_string()));
    }
    let mut uncond_out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (callee, regs) in uncond {
        uncond_out.insert(callee.to_string(), regs.iter().map(|s| s.to_string()).collect());
    }
    check_result_invalid_path(&p.name, &buf.items, &cc_callees, &uncond_out)
}

/// `out(a1 if cc)` — a1 valid only when carry CLEAR. After the call, `bcs .fail`
/// takes the invalid (carry-set) edge; reading a1 there is a
/// `[call.result-invalid-path]`.
#[test]
fn invalid_path_read_fires() {
    let f = run_cond(
        "module m\n\
         proc P () clobbers(d0-d1/a1) {\n\
             jbsr Alloc\n\
             bcs .fail\n\
             move.w (a1), d0\n\
             rts\n\
         .fail:\n\
             move.w (a1), d1\n\
             rts\n\
         }\n",
        "Alloc",
        "a1",
        "cc",
    );
    assert_eq!(f.len(), 1, "reading a1 on the carry-set path is invalid: {f:?}");
    assert!(matches!(f[0].kind, FlagFiringKind::InvalidPathRead { .. }));
}

/// Reading a1 only on the VALID (carry-clear) path is fine — no firing.
#[test]
fn valid_path_read_passes() {
    let f = run_cond(
        "module m\n\
         proc P () clobbers(d0/a1) {\n\
             jbsr Alloc\n\
             bcs .fail\n\
             move.w (a1), d0\n\
             rts\n\
         .fail:\n\
             rts\n\
         }\n",
        "Alloc",
        "a1",
        "cc",
    );
    assert!(f.is_empty(), "a1 read only on the valid path: {f:?}");
}

/// If the invalid path REDEFINES a1 (a fresh `lea`) before any read, a1 is no
/// longer the invalid result — no firing.
#[test]
fn invalid_path_redefine_before_read_passes() {
    let f = run_cond(
        "module m\n\
         proc P () clobbers(d0/a1) {\n\
             jbsr Alloc\n\
             bcs .fail\n\
             rts\n\
         .fail:\n\
             lea Fallback, a1\n\
             move.w (a1), d0\n\
             rts\n\
         }\n",
        "Alloc",
        "a1",
        "cc",
    );
    assert!(f.is_empty(), "a1 rebuilt on the invalid path before use: {f:?}");
}

/// A callee that does NOT declare a flag result is never checked — a plain call
/// followed by a redefine and return is fine.
#[test]
fn non_flag_callee_is_never_checked() {
    let f = run(
        "module m\n\
         proc P () clobbers(d0) {\n\
             jbsr PlainSub\n\
             moveq #0, d0\n\
             rts\n\
         }\n",
        "Queue", // the flag-callee is Queue; PlainSub is not it
        NONE,
    );
    assert!(f.is_empty(), "PlainSub returns no flag — nothing to consume: {f:?}");
}

// --- §6 (A): an intervening UNCONDITIONAL out() kills the conditional taint ----
//
// FillRow's shape: a conditional-out callee, then a call that unconditionally
// re-produces the register, then a read on the invalid edge. The intervening
// unconditional out redefines the register, so the read sees the fresh value —
// NOT the invalid-path trash. Credited via the SHARED `call_unconditional_outs`
// primitive (the same fact must-def uses). GUARDRAIL: a CONDITIONAL intervening
// out must NEVER count as a redefine, or a real invalid-path read ships unflagged.

/// STILL-FIRES: conditional out read on the invalid (!eq) edge with NO intervening
/// redefine → fires. (The baseline the credit must not blind.)
#[test]
fn invalid_path_no_redefine_still_fires() {
    let f = run_invalid(
        "module m\n\
         proc P () clobbers(d0-d1/a1) {\n\
             jbsr Find\n\
             beq .have\n\
             move.w (a1), d1\n\
             rts\n\
         .have:\n\
             move.w (a1), d0\n\
             rts\n\
         }\n",
        &[("Find", "a1", "eq")],
        &[],
    );
    assert_eq!(f.len(), 1, "a1 read on the !eq edge, no redefine → fires: {f:?}");
    assert!(matches!(f[0].kind, FlagFiringKind::InvalidPathRead { .. }));
}

/// NOW-CLEARS: FillRow shape — conditional out, then an intervening UNCONDITIONAL
/// out(a1) (Decomp), then the read → the read sees the fresh a1, no firing.
#[test]
fn invalid_path_uncond_out_redefine_clears() {
    let f = run_invalid(
        "module m\n\
         proc P () clobbers(d0-d1/a1) {\n\
             jbsr Find\n\
             beq .have\n\
             jbsr Decomp\n\
             move.w (a1), d1\n\
             rts\n\
         .have:\n\
             move.w (a1), d0\n\
             rts\n\
         }\n",
        &[("Find", "a1", "eq")],
        &[("Decomp", &["a1"])],
    );
    assert!(f.is_empty(), "Decomp's unconditional out(a1) redefines a1 before the read: {f:?}");
}

/// TRAP (guardrail 1): conditional out, then a CONDITIONAL out on the SAME reg
/// (Find2, NOT unconditional), then the read on the invalid edge → must STILL
/// fire. A conditional out is trash on its own invalid edge and is NEVER a
/// redefine; crediting it would false-NEGATIVE a real invalid-path read.
#[test]
fn invalid_path_conditional_out_does_not_kill_trap() {
    let f = run_invalid(
        "module m\n\
         proc P () clobbers(d0-d1/a1) {\n\
             jbsr Find\n\
             beq .have\n\
             jbsr Find2\n\
             move.w (a1), d1\n\
             rts\n\
         .have:\n\
             move.w (a1), d0\n\
             rts\n\
         }\n",
        &[("Find", "a1", "eq"), ("Find2", "a1", "ne")],
        &[], // Find2 is CONDITIONAL — NOT in the unconditional-out map
    );
    assert!(
        f.iter().any(|x| matches!(&x.kind, FlagFiringKind::InvalidPathRead { reg, .. } if reg == "a1")
            && x.callee == "Find"),
        "Find2's CONDITIONAL out must NOT kill the taint — Find's invalid-path a1 read still fires: {f:?}"
    );

    // MUTATION (proves the guardrail is load-bearing, not decorative): if the fix
    // WRONGLY credited Find2's out as an unconditional redefine, the taint would be
    // killed and the read would ship unflagged — a false NEGATIVE on a live ERROR
    // gate. Passing Find2 in the unconditional-out map simulates exactly that bug.
    let weakened = run_invalid(
        "module m\n\
         proc P () clobbers(d0-d1/a1) {\n\
             jbsr Find\n\
             beq .have\n\
             jbsr Find2\n\
             move.w (a1), d1\n\
             rts\n\
         .have:\n\
             move.w (a1), d0\n\
             rts\n\
         }\n",
        &[("Find", "a1", "eq")],
        &[("Find2", &["a1"])], // <-- the bug: treat the conditional out as unconditional
    );
    assert!(
        weakened.is_empty(),
        "mutation check: crediting Find2's out as unconditional silences the read \
         (this is the false negative the guardrail prevents): {weakened:?}"
    );
}
