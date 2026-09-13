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
        eval_proc_body(&file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[], &sigil_frontend_emp::contract::InterfaceEnv::empty());
    let buf = buf.expect("codebuf");
    let mut fc: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    fc.insert(callee.to_string(), BTreeSet::from(["carry".to_string()]));
    check_flag_unused(&p.name, &buf.items, &fc, discarded, Cpu::M68000)
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
    assert!(f.is_empty(), "carry consumed by bcs, should not fire: {f:?}");
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
    assert!(f.is_empty(), "movem is CC-transparent, carry survives to bcs: {f:?}");
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
    assert_eq!(f.len(), 1, "addx redefines carry before the bcs, must fire: {f:?}");
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
    assert_eq!(f.len(), 1, "move to sr redefines carry before the bcs, must fire: {f:?}");
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
    assert_eq!(f.len(), 1, "return abandons the flag, should fire: {f:?}");
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
    assert_eq!(f.len(), 1, "the .skip path returns unconsumed, should fire: {f:?}");
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
        eval_proc_body(&file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[], &sigil_frontend_emp::contract::InterfaceEnv::empty());
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
    assert!(f.is_empty(), "PlainSub returns no flag, nothing to consume: {f:?}");
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
        "Find2's CONDITIONAL out must NOT kill the taint, Find's invalid-path a1 read still fires: {f:?}"
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

// ---------------------------------------------------------------------------
// rung-2 §13.3 sub-part 2 — the ADDITIVE Z80 arms in the carry consume/redefine
// model + the Z80 terminator/edge model. The 68k allowlists above are
// byte-unchanged; these exercise the Z80 branches through the SAME
// check_flag_unused entry, evaluated under Cpu::Z80. Corpus shape:
// `call PsgVolEnv_Resolve` (declares out(carry: found)) then a `jr c`/`jr nc`
// carry test — the sound_psg.asm:120 demand.
// ---------------------------------------------------------------------------

/// Eval the first proc under Cpu::Z80 and run the flag-unused check for a Z80
/// `call` to `callee` (declared to return carry). The Z80 sibling of [`run`].
fn run_z80(src: &str, callee: &str) -> Vec<FlagFiring> {
    let (file, diags) = parse_str(src);
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {diags:?}");
    let p = file
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) => Some(p),
            _ => None,
        })
        .expect("a proc");
    let (buf, _d, _n) =
        eval_proc_body(&file, &p.name, &p.params, &p.body, p.span, 0, Cpu::Z80, &[], &sigil_frontend_emp::contract::InterfaceEnv::empty());
    let buf = buf.expect("codebuf");
    let mut fc: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    fc.insert(callee.to_string(), BTreeSet::from(["carry".to_string()]));
    check_flag_unused(&p.name, &buf.items, &fc, NONE, Cpu::Z80)
}

/// CONSUMED: a Z80 `jr c` reads the carry result before any redefine — the
/// sound_psg.asm consumer form. No firing (the positive control for the arm).
#[test]
fn z80_jr_c_consumes_carry() {
    let f = run_z80(
        "module m\n\
         proc P () {\n\
             call Resolve\n\
             jr c, .unknown\n\
             ld a, 1\n\
         .unknown:\n\
             ret\n\
         }\n",
        "Resolve",
    );
    assert!(f.is_empty(), "jr c consumes the carry result, should not fire: {f:?}");
}

/// CONSUMED: `jr nc` (carry-clear) is the mirror consumer — also discharges the
/// obligation. No firing.
#[test]
fn z80_jr_nc_consumes_carry() {
    let f = run_z80(
        "module m\n\
         proc P () {\n\
             call Resolve\n\
             jr nc, .found\n\
             ret\n\
         .found:\n\
             ret\n\
         }\n",
        "Resolve",
    );
    assert!(f.is_empty(), "jr nc consumes the carry result, should not fire: {f:?}");
}

/// ABANDONED: a Z80 caller that returns without testing carry drops the result
/// — the flag-result-unused firing (the psg-header bug class).
#[test]
fn z80_return_without_consume_fires() {
    let f = run_z80(
        "module m\n\
         proc P () {\n\
             call Resolve\n\
             ld a, 0\n\
             ret\n\
         }\n",
        "Resolve",
    );
    assert_eq!(f.len(), 1, "return abandons the Z80 carry result, should fire: {f:?}");
    assert_eq!(f[0].flag, "carry");
}

/// REDEFINED: an intervening `scf` (a Z80 carry writer, the new z80_writes_carry
/// arm) between the call and a `jr c` clobbers the callee's carry — the later
/// test reads the wrong flag, so the result is abandoned. Must fire.
#[test]
fn z80_scf_redefines_carry_and_fires() {
    let f = run_z80(
        "module m\n\
         proc P () {\n\
             call Resolve\n\
             scf\n\
             jr c, .x\n\
         .x:\n\
             ret\n\
         }\n",
        "Resolve",
    );
    assert_eq!(f.len(), 1, "scf redefines carry before the jr c, must fire: {f:?}");
}

/// TRANSPARENT: a `jr z` (Zero-testing, NOT carry) between the call and the
/// carry consumer neither consumes nor redefines carry — the carry survives to
/// the `jr c`. No firing (the Z80 analog of the movem-transparency case; proves
/// z80_reads_carry fences on the exact cc, and the two-way `jr z` edge split is
/// walked without abandoning).
#[test]
fn z80_jr_z_is_carry_transparent() {
    let f = run_z80(
        "module m\n\
         proc P () {\n\
             call Resolve\n\
             jr z, .zero\n\
         .zero:\n\
             jr c, .done\n\
         .done:\n\
             ret\n\
         }\n",
        "Resolve",
    );
    assert!(f.is_empty(), "jr z is carry-transparent, carry survives to jr c: {f:?}");
}

// ---------------------------------------------------------------------------
// The zero-flag model (`out(zero: name)`). Its readers and writers come from the
// ISA manuals (M68000PRM Tables 3-18/3-19 and per-instruction condition codes;
// UM0080 "Condition Bits Affected"), not from the carry tables, and several
// instructions sit on opposite sides for the two flags. Each class below is
// driven end to end from `.emp` source through `eval_proc_body`.
// ---------------------------------------------------------------------------

/// Eval the first proc in `src` for `cpu` and run the flag-unused check with
/// `callee` declared to return `flag`, `discarded` the opted-out call spans.
fn run_flag(src: &str, callee: &str, flag: &str, cpu: Cpu, discarded: &[Span]) -> Vec<FlagFiring> {
    let (file, diags) = parse_str(src);
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {diags:?}");
    let p = file
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) => Some(p),
            _ => None,
        })
        .expect("a proc");
    let (buf, _d, _n) = eval_proc_body(
        &file,
        &p.name,
        &p.params,
        &p.body,
        p.span,
        0,
        cpu,
        &[],
        &sigil_frontend_emp::contract::InterfaceEnv::empty(),
    );
    let buf = buf.expect("codebuf");
    let mut fc: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    fc.insert(callee.to_string(), BTreeSet::from([flag.to_string()]));
    check_flag_unused(&p.name, &buf.items, &fc, discarded, cpu)
}

/// A 68k proc that calls `Find` and then runs `body`.
fn m68k_src(body: &str) -> String {
    format!("module m\nproc P () clobbers(d0-d1/a0-a1) {{\n    jbsr Find\n{body}}}\n")
}

/// A Z80 proc that calls `Find` and then runs `body`.
fn z80_src(body: &str) -> String {
    format!("module m\nproc P () {{\n    call Find\n{body}}}\n")
}

fn m68k_zero(body: &str) -> Vec<FlagFiring> {
    run_flag(&m68k_src(body), "Find", "zero", Cpu::M68000, NONE)
}
fn m68k_carry(body: &str) -> Vec<FlagFiring> {
    run_flag(&m68k_src(body), "Find", "carry", Cpu::M68000, NONE)
}
fn z80_zero(body: &str) -> Vec<FlagFiring> {
    run_flag(&z80_src(body), "Find", "zero", Cpu::Z80, NONE)
}
fn z80_carry(body: &str) -> Vec<FlagFiring> {
    run_flag(&z80_src(body), "Find", "carry", Cpu::Z80, NONE)
}

/// Assert `run(body)` fires exactly once on a `zero` result, or not at all.
fn expect(run: fn(&str) -> Vec<FlagFiring>, body: &str, fires: bool, why: &str) {
    let f = run(body);
    if fires {
        assert_eq!(f.len(), 1, "{why}: expected one firing for\n{body}got {f:?}");
    } else {
        assert!(f.is_empty(), "{why}: expected no firing for\n{body}got {f:?}");
    }
}

/// 68k READERS of Z (Table 3-19: EQ, NE, HI, LS, GT, LE include Z), in the
/// Bcc, Scc and DBcc forms, and MOVE from SR, which copies the whole CCR out.
#[test]
fn m68k_zero_is_consumed_by_every_z_testing_condition() {
    for body in [
        "    beq .x\n    moveq #0, d0\n.x:\n    rts\n",
        "    bne .x\n    moveq #0, d0\n.x:\n    rts\n",
        "    bhi .x\n.x:\n    rts\n",
        "    bls .x\n.x:\n    rts\n",
        "    bgt .x\n.x:\n    rts\n",
        "    ble .x\n.x:\n    rts\n",
        "    seq d0\n    rts\n",
        "    sne d0\n    rts\n",
        "    dbne d1, .x\n.x:\n    rts\n",
        "    dbeq d1, .x\n.x:\n    rts\n",
        "    move.w sr, -(sp)\n    rts\n",
    ] {
        expect(m68k_zero, body, false, "a Z reader consumes the zero result");
    }
    let f = m68k_zero("    rts\n");
    assert_eq!(f.len(), 1, "the control: returning unread fires, got {f:?}");
    assert_eq!(f[0].flag, "zero");
}

/// 68k conditions whose test does NOT include Z (CC, CS, GE, LT, PL, MI, VC,
/// VS) neither consume nor redefine it: both edges are walked, so a path
/// returning unread still fires. `bcs` is the asymmetric member: it consumes a
/// carry result and not a zero one.
#[test]
fn m68k_zero_is_not_consumed_by_a_condition_without_z() {
    for cc in ["bcs", "bcc", "bge", "blt", "bpl", "bmi", "bvc", "bvs"] {
        let body = format!("    {cc} .x\n    rts\n.x:\n    rts\n");
        expect(m68k_zero, &body, true, "a condition without Z leaves the zero result unread");
    }
    expect(m68k_carry, "    bcs .x\n    rts\n.x:\n    rts\n", false, "bcs consumes carry");
}

/// 68k WRITERS of Z before the reader end the window: a result-setting data
/// operation, a bit test, CLR, MOVE to CCR / SR, and an intervening call.
#[test]
fn m68k_zero_writers_redefine_it_before_the_beq() {
    for op in [
        "moveq #0, d0",
        "move.w d0, d1",
        "tst.w d0",
        "cmpi.w #1, d0",
        "addq.w #1, d0",
        "clr.w d0",
        "lsl.w #1, d0",
        "btst #0, d0",
        "move.w #0, ccr",
        "move.w #$2700, sr",
        "jbsr Other",
    ] {
        let body = format!("    {op}\n    beq .x\n.x:\n    rts\n");
        expect(m68k_zero, &body, true, "a Z writer redefines the zero result");
    }
}

/// 68k instructions the manual lists as not affecting the condition codes are
/// transparent: MOVEA, LEA, MOVEM, and an address-register destination for
/// MOVE / ADDQ / SUBQ ("the condition codes are not affected when the
/// destination is an address register").
#[test]
fn m68k_zero_survives_instructions_that_leave_the_ccr_alone() {
    for op in [
        "movea.l a0, a1",
        "lea (a0), a1",
        "movem.l (sp)+, d0-d1",
        "move.l d0, a1",
        "addq.l #2, sp",
        "subq.l #4, a0",
    ] {
        let body = format!("    {op}\n    beq .x\n.x:\n    rts\n");
        expect(m68k_zero, &body, false, "an instruction leaving the CCR alone is transparent");
    }
}

/// THE STICKY CASE. ADDX: "Z: Cleared if the result is nonzero; unchanged
/// otherwise." It can only clear Z, so a later reader still reads the callee's
/// value where it survived: the walk goes through it rather than stopping. A
/// path that then returns unread still fires. For carry ADDX is a full writer,
/// so the same body fires there: the asymmetric member.
#[test]
fn m68k_addx_clears_zero_only_and_the_walk_goes_on_through_it() {
    expect(m68k_zero, "    addx.w d0, d1\n    beq .x\n.x:\n    rts\n", false, "addx then beq");
    expect(m68k_zero, "    addx.w d0, d1\n    rts\n", true, "addx then return, unread");
    expect(m68k_zero, "    addx.w d0, d1\n    tst.w d0\n    beq .x\n.x:\n    rts\n", true, "addx then a writer");
    expect(m68k_carry, "    addx.w d0, d1\n    bcs .x\n.x:\n    rts\n", true, "addx writes carry");
}

/// ANDI / ORI / EORI to CCR move Z only as bit 2 of the immediate says: ANDI
/// clears it when bit 2 is zero, ORI sets it and EORI changes it when bit 2 is
/// one, and each leaves it unchanged otherwise. The corpus's own
/// `andi.b #$FE, ccr` (carry clear) leaves Z standing.
#[test]
fn m68k_ccr_immediate_forms_move_zero_by_bit_2() {
    for (op, fires) in [
        ("andi.b #$FE, ccr", false),
        ("andi.b #$FB, ccr", true),
        ("ori.b #$01, ccr", false),
        ("ori.b #$04, ccr", true),
        ("eori.b #$01, ccr", false),
        ("eori.b #$04, ccr", true),
    ] {
        let body = format!("    {op}\n    beq .x\n.x:\n    rts\n");
        expect(m68k_zero, &body, fires, "bit 2 of the CCR immediate");
    }
}

/// BTST writes Z and leaves C alone: it redefines a zero result and is
/// transparent to a carry one. The asymmetric pair, one body per flag.
#[test]
fn m68k_btst_writes_zero_and_not_carry() {
    expect(m68k_zero, "    btst #0, d0\n    beq .x\n.x:\n    rts\n", true, "btst writes Z");
    expect(m68k_carry, "    btst #0, d0\n    bcs .x\n.x:\n    rts\n", false, "btst leaves C");
}

/// Must-use is every-path on the zero result too: the `bcs` taken edge returns
/// before any Z reader, so the call fires though the other path reads Z.
#[test]
fn m68k_zero_one_unconsumed_path_at_a_join_fires() {
    let body = "    bcs .skip\n    beq .done\n.skip:\n    rts\n.done:\n    rts\n";
    expect(m68k_zero, body, true, "the .skip path returns unread");
}

/// `@discards(name)` opts a zero result out exactly as it does a carry one.
#[test]
fn m68k_zero_discards_suppresses_the_firing() {
    let src = "module m\n\
               proc P () clobbers(d0) {\n\
                   jbsr Find @discards(found)\n\
                   moveq #0, d0\n\
                   rts\n\
               }\n";
    let with = run_flag(src, "Find", "zero", Cpu::M68000, &discards_spans(src));
    assert!(with.is_empty(), "@discards must suppress a zero result: {with:?}");
    let without = run_flag(src, "Find", "zero", Cpu::M68000, NONE);
    assert_eq!(without.len(), 1, "the same call fires without the discard span");
}

/// Z80 READERS of Z: the NZ and Z conditions of `jr`, `jp`, `call` and `ret`
/// (the `cc` table, relevant flag Z), and PUSH AF, which copies F out.
#[test]
fn z80_zero_is_consumed_by_the_z_conditions() {
    for body in [
        "    jr z, .x\n.x:\n    ret\n",
        "    jr nz, .x\n    ret\n.x:\n    ret\n",
        "    jp z, .x\n    ret\n.x:\n    ret\n",
        "    ret nz\n    ret\n",
        "    call z, Other\n    ret\n",
        "    push af\n    pop af\n    ret\n",
    ] {
        expect(z80_zero, body, false, "a Z reader consumes the zero result");
    }
    let f = z80_zero("    ret\n");
    assert_eq!(f.len(), 1, "the control: returning unread fires, got {f:?}");
    assert_eq!(f[0].flag, "zero");
}

/// Z80 carry conditions do not read Z: `jr c` consumes a carry result and not a
/// zero one.
#[test]
fn z80_zero_is_not_consumed_by_a_carry_condition() {
    expect(z80_zero, "    jr c, .x\n    ret\n.x:\n    ret\n", true, "jr c leaves Z unread");
    expect(z80_carry, "    jr c, .x\n    ret\n.x:\n    ret\n", false, "jr c consumes carry");
}

/// Z80 WRITERS of Z: 8-bit arithmetic and logic, 8-bit INC / DEC, BIT, the CB
/// rotates, POP AF and EX AF, AF' (which replace F), and an intervening call.
#[test]
fn z80_zero_writers_redefine_it_before_the_jr_z() {
    for op in [
        "add a, b",
        "cp 1",
        "or a",
        "inc a",
        "dec b",
        "inc (hl)",
        "bit 0, a",
        "rlc a",
        "srl a",
        "pop af",
        "ex af, af'",
        "call Other",
    ] {
        let body = format!("    {op}\n    jr z, .x\n.x:\n    ret\n");
        expect(z80_zero, &body, true, "a Z writer redefines the zero result");
    }
}

/// Z80 instructions the manual lists as "Z is not affected" or affecting no
/// condition bit are transparent: `ld`, 16-bit INC / DEC and ADD HL, ss, SCF,
/// CCF, CPL, the accumulator rotates, SET / RES, LDIR, `ex de, hl`.
#[test]
fn z80_zero_survives_instructions_that_leave_z_alone() {
    for op in [
        "ld a, b",
        "inc hl",
        "dec de",
        "add hl, bc",
        "scf",
        "ccf",
        "cpl",
        "rlca",
        "rra",
        "set 0, a",
        "res 1, b",
        "ldir",
        "ex de, hl",
    ] {
        let body = format!("    {op}\n    jr z, .x\n.x:\n    ret\n");
        expect(z80_zero, &body, false, "an instruction leaving Z alone is transparent");
    }
}

/// The Z80 asymmetric members, one body per flag: 8-bit INC and BIT write Z and
/// leave C; SCF, RLCA and ADD HL, ss write C and leave Z.
#[test]
fn z80_zero_and_carry_disagree_where_the_manual_does() {
    for (op, zero_fires, carry_fires) in [
        ("inc a", true, false),
        ("bit 0, a", true, false),
        ("scf", false, true),
        ("rlca", false, true),
        ("add hl, bc", false, true),
    ] {
        expect(z80_zero, &format!("    {op}\n    jr z, .x\n.x:\n    ret\n"), zero_fires, op);
        expect(z80_carry, &format!("    {op}\n    jr c, .x\n.x:\n    ret\n"), carry_fires, op);
    }
}

/// The site census: a `zero` site is walked, a flag with no model (`negative`)
/// is recorded as such and not walked, on the same call.
#[test]
fn a_zero_site_is_walked_and_a_flag_without_a_model_is_not() {
    use sigil_frontend_emp::flag_check::{check_flag_unused_sites, FlagSiteOutcome};
    let src = m68k_src("    beq .x\n.x:\n    rts\n");
    let (file, _d) = parse_str(&src);
    let p = file
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) => Some(p),
            _ => None,
        })
        .expect("a proc");
    let (buf, _d, _n) = eval_proc_body(
        &file,
        &p.name,
        &p.params,
        &p.body,
        p.span,
        0,
        Cpu::M68000,
        &[],
        &sigil_frontend_emp::contract::InterfaceEnv::empty(),
    );
    let buf = buf.expect("codebuf");
    let mut fc: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    fc.insert("Find".to_string(), BTreeSet::from(["zero".to_string(), "negative".to_string()]));
    let (firings, sites) = check_flag_unused_sites(&p.name, &buf.items, &fc, NONE, Cpu::M68000);
    assert!(firings.is_empty(), "the beq consumes Z, and negative is not walked: {firings:?}");
    let outcome = |r: &str| sites.iter().find(|s| s.result == r).map(|s| s.outcome);
    assert_eq!(outcome("zero"), Some(FlagSiteOutcome::Walked));
    assert_eq!(outcome("negative"), Some(FlagSiteOutcome::NoConsumerModel));
}

/// JOIN: one path consumes the carry (`jr c`), the other returns unconsumed —
/// must-use is every-path, so it fires (the Z80 CFG has real joins via
/// z80_edges' two-way conditional split).
#[test]
fn z80_one_unconsumed_path_at_a_join_fires() {
    let f = run_z80(
        "module m\n\
         proc P () {\n\
             call Resolve\n\
             jr z, .skip\n\
             jr c, .done\n\
         .skip:\n\
             ret\n\
         .done:\n\
             ret\n\
         }\n",
        "Resolve",
    );
    assert_eq!(f.len(), 1, "the .skip path returns unconsumed, should fire: {f:?}");
}
