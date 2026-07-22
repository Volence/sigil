//! Lower an [`Item::Proc`](crate::ast::Item::Proc) to Core IR (Spec 2, Plan 4 —
//! T4, §5.1). A proc becomes: a label named after the proc, then its body
//! lowered through the SAME machinery `asm { }` uses — the body is evaluated to
//! a resolved [`CodeBuf`](crate::value::CodeBuf) by
//! [`eval_proc_body`](crate::eval::eval_proc_body) (reusing `eval_asm`) and
//! streamed by [`lower_code_buf`](super::lower_code_buf) (reusing T3's backend
//! dispatch). No instruction lowering is re-implemented here (D-P4.1).
//!
//! T4 also runs three §5.1 proc-contract checks over the resolved body:
//!
//! - **Declared fallthrough** (`falls_into next`): `next` must be the item
//!   IMMEDIATELY following this proc in the section (declaration order) — any
//!   item between them, an out-of-order target, or a non-proc target breaks the
//!   physical fallthrough and is the `[proc.fallthrough-separated]` error.
//! - **Undeclared fallthrough** (default-on warning): a proc with no
//!   `falls_into` whose body can reach its closing `}` without an unconditional
//!   terminator (`rts`/`rte`/`bra`/`jmp`/`jbra` on 68k; `ret`/`jp`/`jr` on Z80) warns
//!   `[proc.undeclared-fallthrough]`. T4's analysis is deliberately minimal — it
//!   inspects only the LAST instruction's mnemonic; the full control-flow
//!   reachability version is deferred (S2-D6/D7).
//! - **`clobbers` lint** (default-on, D-P4.9): a write to a register outside the
//!   declared `clobbers(...)` set ∪ params is `[proc.clobber-undeclared]`. This
//!   is NECESSARILY a heuristic (it is assembly): T4 flags the destination
//!   register operand of the standard write-form mnemonics (`move`, `add`,
//!   `moveq`, `clr`, …). Read-only / control forms (`cmp`, `tst`, `bra`, `jmp`)
//!   and memory-destination writes do not trigger it. The full register-dataflow
//!   contract is the deferred S2-D6 sub-milestone.

use crate::ast;
use crate::eval::eval_proc_body;
use crate::value::{CodeItem, CodeOperand, Reg};
use sigil_ir::backend::{Cpu, IrStreamer};
use sigil_ir::IrBuilder;
use sigil_span::{Diagnostic, Level, Span};
use std::collections::{BTreeSet, HashSet};

/// This proc's position among its declaration-order siblings — the context a
/// declared `falls_into` needs to check physical adjacency (§5.1). Bundling the
/// `(index, items)` pair keeps [`lower_proc`]'s signature within the arg budget
/// and reads as one concept ("where this proc sits").
pub(super) struct Siblings<'a> {
    /// This proc's index within `items`.
    pub index: usize,
    /// The declaration-order item list this proc belongs to (the module's items,
    /// or a `section {}` block's items).
    pub items: &'a [ast::Item],
}

/// How a proc lowers: its CPU (drives code encoding + the terminator table) and
/// whether the enclosing module is `@as_compat` (which silences the heuristic
/// modernization WARNINGs). Bundled so `lower_proc` stays under clippy's
/// argument-count lint (mirroring how [`Siblings`] bundles position).
pub(super) struct ProcCtx<'a> {
    /// The CPU this proc's body encodes for.
    pub cpu: Cpu,
    /// Module-level `@as_compat` — silence the faithful-port lints (D-P6.3).
    pub as_compat: bool,
    /// Comptime `-D NAME=INT` defines (sound-migration T2 Task 1, R1), seeded
    /// into this proc's evaluator so its body can reference one like any
    /// other name.
    pub defines: &'a [(String, i128)],
}

/// Lower one proc: define its label, evaluate + lower its body, then run the
/// §5.1 fallthrough / clobber contract checks. `siblings` locates this proc in
/// declaration order so declared fallthrough can check adjacency. `asm_counter`
/// is the module-wide instantiation counter (D-P4.6): it seeds this proc's
/// evaluator and is advanced by however many `asm { }` bodies it instantiates, so
/// `k` stays globally monotonic across procs (a fresh evaluator per proc would
/// otherwise reset it and collide labels). `as_compat` (module `@as_compat`,
/// Plan 6 D-P6.3) silences the heuristic modernization WARNINGs — undeclared
/// fallthrough and the clobber lint — while leaving the hard `falls_into`
/// adjacency ERROR untouched.
pub(super) fn lower_proc(
    file: &ast::File,
    proc: &ast::ProcDecl,
    siblings: Siblings<'_>,
    ctx: ProcCtx,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
    asm_counter: &mut u32,
) {
    // 1. Label + body → IR. Params emit no code (declarative register bindings).
    builder.define_label(&proc.name);
    // D2.29 amendment: a 68k proc at an odd final address is an address-error
    // crash — error-tier [layout.odd-item] parity check on the proc's label.
    super::record_odd_item_assert(
        file,
        builder,
        ctx.cpu,
        ctx.as_compat,
        super::OddItemKind::Code,
        &proc.name,
        proc.span,
    );
    let (buf, mut ds, next_counter) = eval_proc_body(
        file,
        &proc.name,
        &proc.params,
        &proc.body,
        proc.span,
        *asm_counter,
        ctx.cpu,
        ctx.defines,
    );
    *asm_counter = next_counter;
    diags.append(&mut ds);
    let Some(buf) = buf else { return };
    super::lower_code_buf(&buf, ctx.cpu, ctx.as_compat, builder, diags);

    // 2/3. Fallthrough contract. A declared `falls_into` demands adjacency (a
    // hard ERROR when broken — never silenced); an undeclared but reachable
    // fall-off the end is a modernization WARNING that `@as_compat` silences
    // (Plan 6, D-P6.3: a faithful port opts out of the faithful-port lints).
    match &proc.falls_into {
        Some(next) => {
            check_fallthrough_adjacent(proc, next, siblings.index, siblings.items, diags)
        }
        None if !ctx.as_compat => check_undeclared_fallthrough(proc, &buf, ctx.cpu, diags),
        None => {}
    }

    // 4. Clobbers lint (only when the proc declares a clobber set — the
    // explicit empty `clobbers()` counts: it declares "touches nothing", so
    // every register write is undeclared) — likewise a modernization warning
    // silenced under `@as_compat`.
    if proc.clobbers.is_some() && !ctx.as_compat {
        check_clobbers(proc, &buf, diags);
    }

    // 5. Preserves contract (S2-D6b SYNTACTIC slice): the declared set must
    // match the literal movem save/restore pair. An opt-in declared CONTRACT
    // like `falls_into` — error tier, NOT silenced by `@as_compat` (only the
    // heuristic modernization lints are).
    if !proc.preserves.is_empty() {
        check_preserves(proc, &buf, diags);
    }

    // 6. Output contract (S2-D6e): a declared `out(...)` set. Like `preserves`,
    // an opt-in declared CONTRACT — error/warning tier, NOT silenced by
    // `@as_compat` (only the heuristic modernization lints are). Runs only when
    // a contract is declared (`Some(_)`; the explicit empty `out()` counts —
    // it declares "returns nothing", so any listed register would be moot but
    // the overlap/unwritten checks still apply to whatever IS listed).
    if proc.out.is_some() {
        check_out(proc, &buf, diags);
    }

    // 7. Flag results (`out(carry: name)`) + conditional register results
    // (`out(rN if cc)`) — contract-grammar v2 §6. Validity only (the caller-side
    // must-use check `[call.flag-result-unused]` lives in the whole-corpus walk,
    // since it needs cross-module contract knowledge). Runs whenever a flag /
    // conditional result is declared. Not silenced by `@as_compat` — an opt-in
    // declared contract, like `preserves`/`out`.
    if !proc.out_flags.is_empty() || !proc.out_cond.is_empty() {
        check_out_flags_cond(&proc.name, &proc.out_flags, &proc.out_cond, diags);
    }
}

/// The status flags a `out(carry: name)` result may name — the 68000 CCR bits.
/// `carry` is the sole corpus demand; the rest are accepted for forward use.
const VALID_FLAGS: [&str; 5] = ["carry", "zero", "negative", "overflow", "extend"];

/// The 68000 condition codes a `out(rN if cc)` guard may name (incl. the `hs`/`lo`
/// aliases of `cc`/`cs`). `t`/`f` are legal cc encodings but nonsensical as a
/// result guard, so they are NOT accepted here.
const VALID_CCS: [&str; 16] = [
    "hi", "ls", "cc", "cs", "ne", "eq", "vc", "vs", "pl", "mi", "ge", "lt", "gt", "le", "hs", "lo",
];

/// Validate `out(carry: name)` flag results and `out(rN if cc)` conditional
/// register results (§6): a flag name outside [`VALID_FLAGS`] is
/// `[proc.out-flag-invalid]`; a condition code outside [`VALID_CCS`] or a
/// non-register `reg` is `[proc.out-cond-invalid]`. Both error-tier, mirroring
/// `[proc.out-invalid]`. (The conditional register's `reg` also rides the `out`
/// reglist, so its register-spelling validity is already covered by
/// `[proc.out-invalid]`; here we only police the `cc`.)
fn check_out_flags_cond(
    proc_name: &str,
    flags: &[ast::FlagResult],
    conds: &[ast::CondResult],
    diags: &mut Vec<Diagnostic>,
) {
    for f in flags {
        if !VALID_FLAGS.contains(&f.flag.as_str()) {
            push(
                diags,
                Level::Error,
                f.span,
                format!(
                    "[proc.out-flag-invalid] `{proc_name}` declares `out({}: …)` — `{}` is not a \
                     status flag (expected one of {})",
                    f.flag,
                    f.flag,
                    VALID_FLAGS.join(", "),
                ),
            );
        }
    }
    for c in conds {
        if !VALID_CCS.contains(&c.cc.as_str()) {
            push(
                diags,
                Level::Error,
                c.span,
                format!(
                    "[proc.out-cond-invalid] `{proc_name}` declares `out({} if {})` — `{}` is not a \
                     condition code",
                    c.reg, c.cc, c.cc,
                ),
            );
        }
    }
}

/// `falls_into next` requires `next` to be the item immediately following `proc`
/// in declaration order (§5.1) — otherwise the two procs are not physically
/// adjacent and the fall cannot happen. Any intervening item (proc or data), an
/// out-of-order target, or a non-proc / missing next item is
/// `[proc.fallthrough-separated]`.
fn check_fallthrough_adjacent(
    proc: &ast::ProcDecl,
    next: &str,
    index: usize,
    items: &[ast::Item],
    diags: &mut Vec<Diagnostic>,
) {
    let adjacent = matches!(items.get(index + 1), Some(ast::Item::Proc(p)) if p.name == next);
    if !adjacent {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.fallthrough-separated] `{}` declares `falls_into {next}`, but `{next}` is \
                 not the immediately-following proc in the section — declared fallthrough requires \
                 the two procs to be adjacent (nothing may sit between them)",
                proc.name
            ),
        );
    }
}

/// A proc with no `falls_into` whose body does not end in an unconditional
/// terminator can reach its closing `}` and run into whatever follows —
/// `[proc.undeclared-fallthrough]` (default-on warning, §5.1). T4 inspects only
/// the LAST `Instr` item's mnemonic (conditional branches like `bne` / `jr cc`
/// do NOT terminate); the full reachability analysis is deferred (S2-D6/D7).
fn check_undeclared_fallthrough(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    cpu: Cpu,
    diags: &mut Vec<Diagnostic>,
) {
    if !ends_in_terminator(buf, cpu) {
        push(
            diags,
            Level::Warning,
            proc.span,
            format!(
                "[proc.undeclared-fallthrough] `{}` can reach its closing `}}` without an \
                 unconditional terminator and does not declare `falls_into` — it will run into \
                 whatever follows it",
                proc.name
            ),
        );
    }
}

/// True when the buf's LAST instruction is an unconditional terminator — the
/// shared core of the proc- and dispatch-body fallthrough lints (same
/// last-mnemonic heuristic, S2-D6/D7 defers full reachability). Exposed
/// `pub(super)` so `lower/script.rs`'s `[script.fallthrough]` check (R9b.9) can
/// reuse the very same terminator recognition (D9.6: a script body that reaches
/// its closing `}` without a terminator runs into whatever follows).
pub(super) fn ends_in_terminator(buf: &crate::value::CodeBuf, cpu: Cpu) -> bool {
    buf.items
        .iter()
        .rev()
        .find_map(|it| match it {
            CodeItem::Instr { mnemonic, .. } => Some(mnemonic.as_str()),
            _ => None,
        })
        .is_some_and(|m| is_terminator(m, cpu))
}

/// 9a (R9a.4): a dispatch member's inline body is an anonymous proc with no
/// `falls_into` surface — a body that can reach its closing `}` without an
/// unconditional terminator runs into the next member's body (or whatever
/// follows the dispatch). Member-flavored mirror of
/// [`check_undeclared_fallthrough`]; silenced under `@as_compat` by the caller,
/// like every modernization lint.
pub(super) fn check_member_body_fallthrough(
    table: &str,
    member: &crate::ast::DispatchMember,
    buf: &crate::value::CodeBuf,
    cpu: Cpu,
    diags: &mut Vec<Diagnostic>,
) {
    if !ends_in_terminator(buf, cpu) {
        push(
            diags,
            Level::Warning,
            member.span,
            format!(
                "[dispatch.body-fallthrough] dispatch `{table}` member `{}`'s inline body can \
                 reach its closing `}}` without an unconditional terminator — it will run into \
                 whatever follows it",
                member.name
            ),
        );
    }
}

/// True for an UNCONDITIONAL control-transfer mnemonic that ends straight-line
/// flow. Conditional forms (`bcc`/`bne`, `jr cc`) and calls (`bsr`/`jsr`) are
/// deliberately excluded — they may fall through.
fn is_terminator(mnemonic: &str, cpu: Cpu) -> bool {
    match cpu {
        // `jbra` (emp auto-reaching branch, D2.18) is an unconditional transfer,
        // so it terminates like `bra`/`jmp`; `jbsr` (a call) is deliberately NOT
        // a terminator — control returns, mirroring `bsr`/`jsr`. `illegal`
        // terminates too: it is the S2-D11(e) `todo!`/`unreachable!` trap —
        // straight-line flow never continues past it (the error vector takes
        // over), so a proc ending in a hole must not ALSO warn fallthrough.
        Cpu::M68000 => matches!(mnemonic, "rts" | "rte" | "bra" | "jmp" | "jbra" | "illegal"),
        Cpu::Z80 => matches!(mnemonic, "ret" | "jp" | "jr"),
    }
}

/// Scan the resolved body for register writes outside `clobbers(...)` ∪ params
/// (§5.1, D-P4.9). HEURISTIC: for the standard write-form mnemonics, the
/// destination is the last operand; if it is a `Dn`/`An` not in the allowed set,
/// warn `[proc.clobber-undeclared]`. Non-writing / control mnemonics and
/// memory-destination writes never trigger. The full register-dataflow contract
/// is the deferred S2-D6 sub-milestone.
///
/// This lint is **68k-only**: the write-form set and `Reg` display below are 68k
/// concepts, so a Z80 proc gets no clobber lint (mirroring the CPU asymmetry in
/// [`is_terminator`]). It also assumes param NAMES are register spellings
/// (`a0`/`d2`/…), which is today's model (§5.1); if params ever gain symbolic
/// names bound to registers, a write to that register would false-positive here.
fn check_clobbers(proc: &ast::ProcDecl, buf: &crate::value::CodeBuf, diags: &mut Vec<Diagnostic>) {
    // Expand + validate the clobbers reglist (C1 items 2/6): ranges expand to
    // their register set, and an invalid entry (`clobbers(d9)`/typo) is a loud
    // `[proc.clobber-invalid]` error at THIS site (the primary owner).
    let clob = reglist_expand_checked(
        proc.clobbers.as_deref().unwrap_or(&[]),
        "clobber",
        &proc.name,
        proc.span,
        diags,
    );
    let mut allowed: HashSet<String> = clob.regs;
    // Params are declarative register bindings (§5.1): a write to a param
    // register is part of the proc's own contract, not an undeclared clobber.
    allowed.extend(proc.params.iter().map(|(name, _, _)| name.clone()));
    // Output registers (S2-D6e) are RESULTS: the proc writes them for the
    // caller to read, so a write to one is part of the contract, not an
    // undeclared clobber. THIS is the immediate win — it silences
    // clobber-undeclared on every declared output register. (Whether such a
    // register is actually written is a SEPARATE concern: `check_out`'s
    // `[proc.out-unwritten]` catches a declared-but-never-written output.)
    // `out`'s own validation runs in `check_out`, so expand it quietly here.
    let outs = reglist_set_quiet(proc.out.as_deref().unwrap_or(&[]));
    allowed.extend(outs.regs);

    for item in &buf.items {
        let CodeItem::Instr { mnemonic, ops, span, .. } = item else { continue };
        // An SR destination is a machine-state clobber (tranche 5): undeclared
        // unless the contract carries `clobbers(sr)`/`out(sr)` or `preserves(sr)`
        // (the latter's balance is checked separately). Only a write-form
        // mnemonic can target SR.
        if writes_dest_register(mnemonic)
            && matches!(ops.last(), Some(CodeOperand::Sr))
            && !clob.has_sr
            && !outs.has_sr
            && !proc.preserves.iter().any(|(lo, hi)| lo == "sr" && hi.is_none())
        {
            push(
                diags,
                Level::Warning,
                *span,
                format!(
                    "[proc.sr-undeclared] `{}` writes `sr` (interrupt mask / condition \
                     codes), which is not in its contract — declare `clobbers(sr)`, or \
                     `preserves(sr)` if the body save/restores it",
                    proc.name
                ),
            );
            continue;
        }
        // The written registers (write-form destination + — after the auto-inc
        // fix — `(An)+`/`-(An)` bases). Reuse `Reg`'s `Display` for the
        // canonical `d0`..`a7` spelling.
        for r in instr_written_regs(mnemonic, ops) {
            // Stack DISCIPLINE on a7 is not a register clobber — every
            // push/pop-balancing proc adjusts sp, and balanced-stack
            // verification is S2-D7(b)'s dataflow job. Two forms: ARITHMETIC
            // (`addq.l #2, sp` / `lea N(sp), sp` cleanup) and PUSH/POP
            // (`move.l x, -(sp)` / `(sp)+`, now that auto-inc/dec advances of
            // a7 are detected). Stack REPLACEMENT (`movea.l x, sp` — switching
            // stacks) stays a genuine a7 clobber and is NOT exempt (tranche-3
            // review scoping).
            if r == crate::value::Reg::A7 && is_sp_discipline(mnemonic, ops) {
                continue;
            }
            let name = r.to_string();
            if !allowed.contains(name.as_str()) {
                push(
                    diags,
                    Level::Warning,
                    *span,
                    format!(
                        "[proc.clobber-undeclared] `{}` writes `{name}`, which is not in its \
                         `clobbers(...)` set or parameter list (heuristic lint — full register \
                         dataflow is deferred to S2-D6)",
                        proc.name
                    ),
                );
            }
        }
    }
}

/// The standard 68k write-form mnemonics whose LAST operand is the written
/// destination, plus the `s<cc>` family (`seq`/`sne`/`spl`/…, all `Scc` — they
/// set a byte in their sole operand). Read-only / control forms (`cmp`, `tst`,
/// `btst`, `bra`, `bsr`, `jmp`, `jsr`, `pea`, `nop`, `rts`…) are absent by
/// design so they never trip the lint.
///
/// `dbcc`/`dbra`/`dbf` decrement their FIRST operand, not the last, so the
/// "destination is the last operand" model does not hold for them — they are
/// covered by [`instr_written_regs`] effect (3) directly, NOT via this
/// last-operand list (S2-D6, closing the tranche-4 dbcc blind spot).
///
/// This is a PARALLEL string list the compiler cannot keep honest against the
/// ISA `Mnemonic` set: a newly-supported write-form (`bchg`, `roxl`, …) will
/// silently escape the lint until it is added HERE. Keep this in sync as the
/// backend's mnemonic table grows. Heuristic (see [`check_clobbers`]).
fn writes_dest_register(m: &str) -> bool {
    matches!(
        m,
        "move"
            | "movea"
            | "moveq"
            | "add"
            | "adda"
            | "addi"
            | "addq"
            | "addx"
            | "sub"
            | "suba"
            | "subi"
            | "subq"
            | "and"
            | "andi"
            | "or"
            | "ori"
            | "eor"
            | "eori"
            | "lea"
            | "clr"
            | "neg"
            | "not"
            | "swap"
            | "ext"
            | "muls"
            | "mulu"
            | "asl"
            | "asr"
            | "lsl"
            | "lsr"
            | "rol"
            | "ror"
            | "bset"
            | "bclr"
            | "tas"
    ) || is_scc(m)
}

/// Every REGISTER this instruction modifies, per the clobber/out write
/// heuristic — the single shared detector behind `check_clobbers`,
/// `check_out`, and the contract census ([`proc_written_registers`]). Two
/// disjoint effects:
///
/// 1. **Write-form destination.** For a [`writes_dest_register`] mnemonic whose
///    LAST operand is a register (`move x, d3` / `lea T, a0`), that register is
///    written. (An SR/CCR/memory destination is not a register write and is
///    handled by the callers separately.)
/// 2. **Auto-increment / -decrement address modification.** `(An)+` and `-(An)`
///    ADVANCE `An` as a side effect regardless of operand position (source OR
///    destination) and regardless of the mnemonic — `move.w (a4)+, d0` writes
///    BOTH `d0` (dest) and `a4` (post-increment); `tst.w (a0)+` writes `a0`
///    even though `tst` is read-only. This closes the auto-inc/dec
///    write-analysis gap ([out-clause, 2026-07-11] gap-ledger row): a pointer
///    result advanced only through `(a4)+` is a genuine write of `a4`, so it
///    can be declared `out(a4)` without a false `[proc.out-unwritten]`, and a
///    proc that scribbles `a4` via `(a4)+` no longer escapes
///    `[proc.clobber-undeclared]`. `a7` via `(sp)+`/`-(sp)` (push/pop) is stack
///    discipline — reported here for honesty but exempted by `check_clobbers`.
/// 3. **`dbcc`-family loop counter (S2-D6).** `dbf`/`dbra`/`dbeq`/… `dN, <label>`
///    DECREMENTS its first-operand data register `dN`; the "destination is the
///    last operand" model (effect 1) does not hold for it, so it is handled
///    explicitly. Closes the tranche-4 "dbcc clobber-lint blind spot": the write
///    set the closure's `local_writes` trusts no longer misses a counter register.
///    (Live corpus impact 0 — every `dbf` counter is `moveq`-initialized first,
///    already counted — but a completeness hole in an ERROR gate's input.)
/// 4. **`movem`-LOAD register list (S2-D6).** `movem <ea>, <reglist>` (reglist =
///    LAST operand = destination, e.g. `movem.l (a0)+, d0-d6/a2`) WRITES every
///    listed register (fresh values). **CLOBBER-LINT POLARITY — read before
///    touching:** a `(sp)+` stack RESTORE (`movem.l (sp)+, d0-d7`) is EXEMPTED
///    (its reglist is preserve-discipline, the direct parallel of the `a7`
///    push/pop exemption in effect 2 — counting a restored reglist would
///    false-positive a defensive over-save `movem d0-d7,-(sp)…(sp)+,d0-d7
///    clobbers(d0-d3)` into a d4-d7 clobber). So this detector is NOT ISA-true
///    for a `(sp)+` movem — it deliberately omits the restored reglist. **Any
///    consumer needing ISA-true movem-load semantics must mask-expand the reglist
///    ITSELF and not rely on this function.** The current such consumers already
///    do exactly that (and dedupe against this): `out_verify::produced_regs`,
///    `calls::written_names`, `preserves::ever_clobbered`, and
///    `preserves::transfer` (whose `is_pop` early-return handles the stack case
///    before it ever reaches this detector). The polarity lives here (not in a
///    caller) because `check_clobbers` consumes this directly for per-span
///    diagnostics and cannot route through `proc_written_registers`.
///
/// Registers are returned in encounter order (dest first, then operand order),
/// DEDUPED (an instruction that advances the same register twice reports it
/// once, so `check_clobbers` does not double-warn at one span). Still a
/// heuristic (this is assembly): the full register-dataflow contract is the
/// deferred S2-D6 sub-milestone.
pub(crate) fn instr_written_regs(mnemonic: &str, ops: &[CodeOperand]) -> Vec<Reg> {
    let mut regs: Vec<Reg> = Vec::new();
    // (1) Write-form destination register (last operand).
    if writes_dest_register(mnemonic) {
        if let Some(CodeOperand::Reg(r)) = ops.last() {
            regs.push(*r);
        }
    }
    // (2) Auto-inc/dec base registers — ANY operand position, ANY mnemonic.
    for op in ops {
        if let CodeOperand::PostInc(r) | CodeOperand::PreDec(r) = op {
            regs.push(*r);
        }
    }
    // (3) dbcc-family counter (first operand, a data register). `db<cc>` is the
    // only mnemonic family spelled `db*`, and the push is further gated on the
    // first operand being a register — matching the `starts_with("db")`
    // convention `flag_check`/`out_verify` already use.
    if mnemonic.starts_with("db") {
        if let Some(CodeOperand::Reg(r)) = ops.first() {
            regs.push(*r);
        }
    }
    // (4) movem-LOAD reglist (last operand = RegList destination), EXCEPT a
    // `(sp)+` stack restore (preserve-discipline exemption — see the doc above).
    if let Some(CodeOperand::RegList(mask)) = ops.last() {
        if !matches!(ops.first(), Some(CodeOperand::PostInc(Reg::A7))) {
            regs.extend(crate::preserves::expand_mask(*mask));
        }
    }
    // Dedup (order-preserving): one instruction may advance the same register
    // twice (`move.w (a0)+, (a0)+`) — report it once so `check_clobbers` does
    // not emit two identical warnings at one span.
    let mut seen = Vec::new();
    regs.retain(|r| if seen.contains(r) { false } else { seen.push(*r); true });
    regs
}

/// The union write set over a resolved body — "the lint's computed write set"
/// (§5.1). The contract census consumes this verbatim to diff a proc's
/// declared `clobbers`/`out` against what it actually writes; `check_out`
/// builds its own `written` set from the same [`instr_written_regs`] detector,
/// so the two never drift. Register spellings are canonical (`d0`..`a7`,
/// `sp`→`a7`) and sorted (BTreeSet) for a deterministic report.
pub fn proc_written_registers(buf: &crate::value::CodeBuf) -> BTreeSet<String> {
    let mut written = BTreeSet::new();
    for item in &buf.items {
        if let CodeItem::Instr { mnemonic, ops, .. } = item {
            for r in instr_written_regs(mnemonic, ops) {
                written.insert(r.to_string());
            }
        }
    }
    written
}

/// True for the `s<cc>` (set-on-condition) spelling: an `s` prefix followed by a
/// known 68k condition code. `Scc` writes a byte to its sole (last) operand, so
/// it belongs in the write-form set. A prefix check is used rather than listing
/// all 16 spellings, and it does not collide with the `s`-initial arithmetic
/// mnemonics (`sub*`/`swap`) — none of their tails is a condition code.
fn is_scc(m: &str) -> bool {
    m.strip_prefix('s').is_some_and(is_condition_code)
}

/// The 68k condition-code suffixes (mirrors the backend's `m68k_cond`, incl. the
/// `hs`/`lo` unsigned aliases). Used only to recognize the `s<cc>` write-form.
fn is_condition_code(cc: &str) -> bool {
    matches!(
        cc,
        "t" | "f"
            | "hi"
            | "ls"
            | "cc"
            | "cs"
            | "hs"
            | "lo"
            | "ne"
            | "eq"
            | "vc"
            | "vs"
            | "pl"
            | "mi"
            | "ge"
            | "lt"
            | "gt"
            | "le"
    )
}

/// Verify the declared `preserves(...)` set against the literal movem
/// save/restore pair (S2-D6b, the SYNTACTIC slice — no dataflow; the full
/// register-contract batch stays gated on S2-D6). The rule: the body's FIRST
/// `movem <list>, -(sp)` and LAST `movem (sp)+, <list>` must both exist (save
/// before restore) and both lists must equal the declared set exactly.
/// A proc that preserves registers some other way (individual pushes) cannot
/// declare `preserves` yet — a missing pair is an error, not a shrug, because
/// a wrong contract is worse than none.
///
/// 68k-only, like the clobber lint (movem/`sp` are 68k concepts); a Z80 proc
/// declaring `preserves` gets the missing-pair error, which is honest — the
/// slice cannot verify it.
fn check_preserves(proc: &ast::ProcDecl, buf: &crate::value::CodeBuf, diags: &mut Vec<Diagnostic>) {
    // Fold the declared segments to the canonical movem mask
    // (bit0=D0..bit7=D7, bit8=A0..bit15=A7 — the `CodeOperand::RegList`
    // convention).
    let mut declared: u16 = 0;
    let mut bad = false;
    let mut preserves_sr = false;
    for (lo, hi) in &proc.preserves {
        // `sr` is contract vocabulary since tranche 5 (S2-D7's first slice):
        // not a movem register, so it rides its OWN save/restore-balance
        // check below instead of the mask fold. Alone only — `sr` in a
        // range falls through to the invalid-register error.
        if lo == "sr" && hi.is_none() {
            preserves_sr = true;
            continue;
        }
        // `ccr` contracts are flag-LIVENESS territory (nearly every
        // instruction writes CCR) — that is S2-D7's dataflow half, not this
        // syntactic slice. Steer rather than pretend.
        if lo == "ccr" && hi.is_none() {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.preserves-invalid] `{}` declares `preserves(ccr)` — CCR contracts \
                     need flag-liveness dataflow (S2-D7), not the syntactic slice; only `sr` \
                     is declarable today",
                    proc.name
                ),
            );
            bad = true;
            continue;
        }
        let Some(lo_bit) = preserves_reg_bit(lo) else {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.preserves-invalid] `{}` declares `preserves({lo}{})` — `{lo}` is \
                     not a register (d0-d7/a0-a7/sp)",
                    proc.name,
                    hi.as_deref().map(|h| format!("-{h}")).unwrap_or_default(),
                ),
            );
            bad = true;
            continue;
        };
        let hi_bit = match hi {
            None => lo_bit,
            Some(h) => match preserves_reg_bit(h) {
                Some(b) if b >= lo_bit => b,
                Some(_) => {
                    push(
                        diags,
                        Level::Error,
                        proc.span,
                        format!(
                            "[proc.preserves-invalid] `{}` declares the reversed range \
                             `{lo}-{h}` — a reglist range runs low to high",
                            proc.name
                        ),
                    );
                    bad = true;
                    continue;
                }
                None => {
                    push(
                        diags,
                        Level::Error,
                        proc.span,
                        format!(
                            "[proc.preserves-invalid] `{}` declares `preserves({lo}-{h})` — \
                             `{h}` is not a register (d0-d7/a0-a7/sp)",
                            proc.name
                        ),
                    );
                    bad = true;
                    continue;
                }
            },
        };
        for bit in lo_bit..=hi_bit {
            declared |= 1 << bit;
        }
    }
    if bad {
        return;
    }

    // A register cannot be both preserved and clobbered — a contradictory
    // contract is diagnosed, not resolved. Expand the clobbers reglist quietly
    // (C1 item 2 — `check_clobbers` owns its diagnostics). (`sr` first: it has
    // no mask bit.)
    let clob = reglist_set_quiet(proc.clobbers.as_deref().unwrap_or(&[]));
    if preserves_sr && clob.has_sr {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.preserves-clobbers-overlap] `{}` declares `sr` both preserved and \
                 clobbered — a register cannot be in both sets",
                proc.name
            ),
        );
        return;
    }
    for c in &clob.regs {
        if let Some(bit) = preserves_reg_bit(c) {
            if declared & (1 << bit) != 0 {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.preserves-clobbers-overlap] `{}` declares `{c}` both preserved \
                         and clobbered — a register cannot be in both sets",
                        proc.name
                    ),
                );
                return;
            }
        }
    }

    if preserves_sr {
        check_preserves_sr(proc, buf, diags);
    }
    if declared == 0 {
        return; // sr-only contract: no movem pair to demand
    }

    // §5 verified preserves (the dataflow upgrade — subsumes the D2.32 movem-pair
    // slice, which becomes its trivial fast path). Every declared register must be
    // provably preserved by symbolic stack tracking ([`crate::preserves`]): its
    // ENTRY value restored on every return path (individual push/pop, `(sp)` peek,
    // mid-body or entry/exit movem, or a superset save), or never written. A
    // register that is NOT provably preserved — a soundness bailout (computed sp,
    // sp escape, aliasing store, stack underflow), a `.w` restore (sign-extends),
    // or a genuine clobber on some exit — is `[proc.preserves-unverifiable]`
    // (error: a wrong contract is worse than none, the D2.32 principle kept).
    // `[proc.preserves-missing-pair]`/`-mismatch`/`-word-pair` retire, subsumed.
    let regs = crate::preserves::expand_mask(declared);
    let status = crate::preserves::verify_preserved(&buf.items, &regs);
    let unverifiable: Vec<String> = regs
        .iter()
        .filter(|r| {
            !matches!(status.get(r), Some(crate::preserves::PreserveStatus::Verified))
        })
        .map(|r| r.to_string())
        .collect();
    if !unverifiable.is_empty() {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.preserves-unverifiable] `{}` declares `preserves({})` but {} not \
                 provably preserved — no save/restore round-trips {} entry value on every \
                 return path (individual push/pop, `movem.l` pair, or `(sp)` peek), an \
                 unmodeled sp op blocks the proof, or a `.w` restore sign-extends and \
                 preserves nothing",
                proc.name,
                mask_reglist(declared),
                unverifiable.join(", "),
                if unverifiable.len() == 1 { "its" } else { "their" },
            ),
        );
    }
}

/// Verify a declared `out(...)` set (S2-D6e — the third register-contract
/// partition member: returned results, beside `clobbers`' scratch and
/// `preserves`' untouched). Four checks, mirroring the `preserves` tiers:
///
/// - `[proc.out-invalid]` (ERROR) — a listed name that is not a register
///   spelling (`d0-d7`/`a0-a7`/`sp`), mirroring `[proc.preserves-invalid]`.
/// - `[proc.out-clobbers-overlap]` / `[proc.out-preserves-overlap]` (ERROR) — a
///   register in BOTH `out` and (`clobbers` | `preserves`) is a contradiction
///   (returned-and-scratch / returned-and-untouched). Preserves segments are
///   expanded to their register set for the membership test.
/// - `[proc.out-unwritten]` (WARN) — an `out`-declared register never written
///   on any path in the body is a false output claim (a stale `out()` after a
///   refactor). The dual of `[proc.clobber-undeclared]`; reuses the SAME
///   register-write detection (`writes_dest_register` → last-operand register
///   destination). Note this is a SEPARATE concern from the register being in
///   `check_clobbers`' `allowed` set: an output is allowed-to-write there AND
///   must-be-written here.
///
/// Register spelling is validated per name (unlike `preserves`, `out` names are
/// never ranges — D-out.1), so an invalid name is reported once and excluded
/// from the overlap/unwritten checks (a nonsense name has no meaningful set
/// membership). 68k + Z80 (D-out.5): outputs are a general calling-convention
/// concept, so this runs for both CPUs; the unwritten check reuses the 68k
/// write-form heuristic, which on Z80 simply finds no matching writes (a Z80
/// `out` currently cannot be verified-written — honest, like `preserves`).
fn check_out(proc: &ast::ProcDecl, buf: &crate::value::CodeBuf, diags: &mut Vec<Diagnostic>) {
    // Expand + validate the out reglist (C1 items 2/6): ranges (`out(d0-d1)`)
    // expand to their register set; a nonsense name is `[proc.out-invalid]` and
    // dropped from the downstream membership checks. Sorted for deterministic
    // diagnostic order.
    let out_set = reglist_expand_checked(
        proc.out.as_deref().unwrap_or(&[]),
        "out",
        &proc.name,
        proc.span,
        diags,
    );
    let mut valid: Vec<String> = out_set.regs.into_iter().collect();
    valid.sort();

    // out ∩ clobbers — returned AND scratch is contradictory. Expand the
    // clobbers reglist quietly (`check_clobbers` owns its diagnostics).
    let clobbers = reglist_set_quiet(proc.clobbers.as_deref().unwrap_or(&[]));
    for name in &valid {
        if clobbers.regs.contains(name) {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.out-clobbers-overlap] `{}` declares `{name}` both output and \
                     clobbered — a register is either a returned result or destroyed scratch, \
                     not both",
                    proc.name
                ),
            );
        }
    }

    // out ∩ preserves — returned AND untouched is contradictory. Preserves
    // stores movem-reglist segments (`(lo, Option<hi>)`): expand each to the
    // canonical mask bits and test the single output register's bit.
    let mut preserved_mask: u16 = 0;
    for (lo, hi) in &proc.preserves {
        let Some(lo_bit) = preserves_reg_bit(lo) else { continue };
        let hi_bit = match hi {
            None => lo_bit,
            Some(h) => match preserves_reg_bit(h) {
                Some(b) if b >= lo_bit => b,
                _ => continue,
            },
        };
        for bit in lo_bit..=hi_bit {
            preserved_mask |= 1 << bit;
        }
    }
    for name in &valid {
        if let Some(bit) = preserves_reg_bit(name) {
            if preserved_mask & (1 << bit) != 0 {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.out-preserves-overlap] `{}` declares `{name}` both output and \
                         preserved — a register is either a returned result or left untouched, \
                         not both",
                        proc.name
                    ),
                );
            }
        }
    }

    // out-unwritten — a declared output never written on any path is a false
    // claim. Same write detection as check_clobbers (the shared
    // [`instr_written_regs`] detector via [`proc_written_registers`]).
    let written = proc_written_registers(buf);
    for name in &valid {
        if !written.contains(name.as_str()) {
            push(
                diags,
                Level::Warning,
                proc.span,
                format!(
                    "[proc.out-unwritten] `{}` declares `out({name})` but never writes `{name}` \
                     — a declared output register must be written (a false result claim, or a \
                     stale `out()` after a refactor)",
                    proc.name
                ),
            );
        }
    }
}

/// The `preserves(sr)` save/restore-balance check (tranche 5 — S2-D7's first
/// syntactic slice; Sound_PostByte is the exhibit): if the body writes SR at
/// all, a `move.w sr, -(sp)` save must precede the FIRST SR write and the
/// LAST SR write must be the `move.w (sp)+, sr` restore. Static order only —
/// no path analysis (a save/restore split across branches is S2-D7's
/// dataflow half); a body with NO SR writes preserves vacuously.
fn check_preserves_sr(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::value::{CodeOperand, Reg};
    let mut first_save: Option<usize> = None;
    let mut sr_writes: Vec<(usize, bool)> = Vec::new(); // (index, is_restore)
    for (i, item) in buf.items.iter().enumerate() {
        let CodeItem::Instr { ops, .. } = item else { continue };
        // Save: `move.w sr, -(sp)` (reads SR — not an SR write).
        if matches!(ops.as_slice(), [CodeOperand::Sr, CodeOperand::PreDec(Reg::A7)]) {
            first_save.get_or_insert(i);
            continue;
        }
        // Any SR-destination form is an SR write; the restore is the
        // `move.w (sp)+, sr` spelling specifically.
        if matches!(ops.last(), Some(CodeOperand::Sr)) {
            let is_restore =
                matches!(ops.as_slice(), [CodeOperand::PostInc(Reg::A7), CodeOperand::Sr]);
            sr_writes.push((i, is_restore));
        }
    }
    let Some(&(first_write, _)) = sr_writes.first() else {
        return; // no SR writes — vacuously preserved
    };
    let saved_before = first_save.is_some_and(|s| s < first_write);
    let restored_last = sr_writes.last().is_some_and(|&(_, r)| r);
    if !(saved_before && restored_last) {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.preserves-sr-unbalanced] `{}` declares `preserves(sr)` but its body's \
                 SR writes are not bracketed by the `move.w sr, -(sp)` … `move.w (sp)+, sr` \
                 pair (the syntactic slice checks static order; path-sensitive save/restore \
                 is S2-D7)",
                proc.name
            ),
        );
    }
}

/// True when an a7 write is stack DISCIPLINE (exempt from the clobber lint)
/// rather than stack REPLACEMENT: either sp arithmetic ([`is_sp_arithmetic`])
/// or a push/pop that advances a7 via `(sp)+`/`-(sp)`. Stack replacement stays a
/// genuine clobber (tranche-3 scoping) — including `movea.l x, sp` (a bare-a7
/// destination) AND the subtle `movea.l (sp)+, sp` (pop INTO sp), where the same
/// instruction both pops (a7 auto-inc, discipline) and loads a new SP (a bare-a7
/// destination). The auto-inc exemption must therefore NOT fire when a7 is also
/// the instruction's bare-register destination.
fn is_sp_discipline(mnemonic: &str, ops: &[CodeOperand]) -> bool {
    if is_sp_arithmetic(mnemonic, ops) {
        return true;
    }
    // A bare-a7 destination is stack REPLACEMENT — not exempt, even alongside a
    // `(sp)+`/`-(sp)` operand on the same instruction.
    let a7_is_dest =
        writes_dest_register(mnemonic) && matches!(ops.last(), Some(CodeOperand::Reg(Reg::A7)));
    !a7_is_dest
        && ops
            .iter()
            .any(|op| matches!(op, CodeOperand::PostInc(Reg::A7) | CodeOperand::PreDec(Reg::A7)))
}

/// True for an sp-DESTINATION write that is stack arithmetic rather than
/// stack replacement: the add/sub immediate families (`addq #2, sp`), or a
/// `lea` whose SOURCE is a displacement over sp itself (`lea N(sp), sp` —
/// the classic frame cleanup). `move`/`movea`-to-sp (stack switching) and
/// `lea Table, sp` do not qualify — those genuinely replace the stack.
fn is_sp_arithmetic(mnemonic: &str, ops: &[CodeOperand]) -> bool {
    match mnemonic {
        "add" | "adda" | "addi" | "addq" | "addx" | "sub" | "suba" | "subi" | "subq"
        | "subx" => true,
        "lea" => matches!(
            ops.first(),
            Some(CodeOperand::DispInd { reg: crate::value::Reg::A7, .. })
        ),
        _ => false,
    }
}

/// A register spelling to its canonical movem-mask bit (bit0=D0..bit7=D7,
/// bit8=A0..bit15=A7), via the shared spelling→register map (so `sp` works).
fn preserves_reg_bit(name: &str) -> Option<u8> {
    use crate::value::Reg;
    let r = Reg::from_name(name)?;
    let (is_a, n) = super::code::reg_kind(r);
    Some(if is_a { 8 + n } else { n })
}

/// A canonical movem-mask bit back to its register spelling (`d0`..`d7`,
/// `a0`..`a7`) — the inverse of [`preserves_reg_bit`], for expanding a range.
fn reg_bit_name(bit: u8) -> String {
    if bit < 8 { format!("d{bit}") } else { format!("a{}", bit - 8) }
}

/// The expansion of a `clobbers`/`out` reglist (C1 items 2 + 6): the canonical
/// register-name SET plus whether `sr` was declared (`sr` is machine state, not
/// a movem register, so it rides its own set rather than a mask bit).
#[derive(Default)]
struct RegSet {
    regs: std::collections::HashSet<String>,
    has_sr: bool,
}

/// Expand + validate a `clobbers`/`out` reglist (C1 item 2 = ranges, item 6 =
/// validation), calling `on_error` with a human reason for each invalid segment.
/// Each segment is a single register (`sr` composes), or an inclusive `lo-hi`
/// movem range; a range endpoint that is not a movem register, or a reversed
/// range, is an error. Canonical names (`sp`→`a7`) so the set matches the
/// `Reg::Display` spelling `check_clobbers`/`check_out` compare against.
fn reglist_expand(segs: &[(String, Option<String>)], mut on_error: impl FnMut(String)) -> RegSet {
    let mut set = RegSet::default();
    for (lo, hi) in segs {
        match hi {
            // A single register or `sr`.
            None => {
                if lo == "sr" {
                    set.has_sr = true;
                } else if let Some(bit) = preserves_reg_bit(lo) {
                    set.regs.insert(reg_bit_name(bit));
                } else {
                    on_error(format!("`{lo}` is not a register (d0-d7/a0-a7/sp) or `sr`"));
                }
            }
            // An inclusive `lo-hi` movem range (`sr` cannot appear in a range).
            Some(h) => {
                let (Some(lo_bit), Some(hi_bit)) = (preserves_reg_bit(lo), preserves_reg_bit(h))
                else {
                    on_error(format!(
                        "`{lo}-{h}` has a non-register endpoint (a range runs d0-d7/a0-a7/sp)"
                    ));
                    continue;
                };
                if hi_bit < lo_bit {
                    on_error(format!("the range `{lo}-{h}` is reversed — a reglist range runs low to high"));
                    continue;
                }
                for bit in lo_bit..=hi_bit {
                    set.regs.insert(reg_bit_name(bit));
                }
            }
        }
    }
    set
}

/// [`reglist_expand`] that emits `[proc.{tag}-invalid]` diagnostics for each bad
/// segment (the primary validation site — C1 item 6).
fn reglist_expand_checked(
    segs: &[(String, Option<String>)],
    tag: &str,
    proc_name: &str,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) -> RegSet {
    reglist_expand(segs, |reason| {
        push(
            diags,
            Level::Error,
            span,
            format!("[proc.{tag}-invalid] `{proc_name}` declares an invalid `{tag}` register: {reason}"),
        )
    })
}

/// [`reglist_expand`] with errors DISCARDED — for a secondary reader of an
/// attribute whose diagnostics another check already owns (so a bad register is
/// reported once, not thrice).
fn reglist_set_quiet(segs: &[(String, Option<String>)]) -> RegSet {
    reglist_expand(segs, |_| {})
}

/// Expand a `clobbers`/`out`/`preserves` reglist to its canonical register-name
/// SET (`d0`..`a7`), silently DROPPING `sr` (the transitive closure is
/// register-file only — `sr` stays the local `[proc.sr-undeclared]` check).
/// Reused by the corpus contract walk ([`crate::corpus_contracts`]) so its
/// declared-register sets match `check_clobbers`' exactly.
pub fn expand_reglist_regs(segs: &[(String, Option<String>)]) -> BTreeSet<String> {
    reglist_set_quiet(segs).regs.into_iter().collect()
}

/// The register set a proc PROVABLY preserves — its declared `preserves` when
/// that set passes the D2.32 syntactic (movem-pair) verification, else empty.
/// The contract closure's `verifiedPreserved(P)` (§1): a register the proc
/// writes but save/restores does not escape it. Reuses [`check_preserves`]
/// verbatim (runs it against a throwaway diag sink) so the "verified" judgment
/// can never drift from the lint — a declared-but-UNVERIFIABLE `preserves`
/// (individual-push, wrong movem) yields the empty set (it stays a D2.32 error
/// at its own site and subtracts nothing). `sr` is dropped (out of the
/// register-file closure's scope).
pub fn verified_preserves_regs(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
) -> BTreeSet<String> {
    if proc.preserves.is_empty() {
        return BTreeSet::new();
    }
    let mut sink = Vec::new();
    check_preserves(proc, buf, &mut sink);
    if sink.iter().any(|d| matches!(d.level, Level::Error)) {
        BTreeSet::new()
    } else {
        expand_reglist_regs(&proc.preserves)
    }
}

/// Format a canonical movem mask back to its reglist spelling (`d0-d1/a0`) —
/// consecutive runs collapse to ranges, data registers before address
/// registers, `a7` spelled `a7`. Diagnostic-only (the inverse of the declared
/// fold, for naming masks in messages).
fn mask_reglist(mask: u16) -> String {
    let mut segs: Vec<String> = Vec::new();
    for (base, prefix) in [(0u8, 'd'), (8u8, 'a')] {
        let mut bit = 0u8;
        while bit < 8 {
            if mask & (1 << (base + bit)) == 0 {
                bit += 1;
                continue;
            }
            let start = bit;
            while bit + 1 < 8 && mask & (1 << (base + bit + 1)) != 0 {
                bit += 1;
            }
            segs.push(if start == bit {
                format!("{prefix}{start}")
            } else {
                format!("{prefix}{start}-{prefix}{bit}")
            });
            bit += 1;
        }
    }
    segs.join("/")
}

/// Push a diagnostic at `span`.
fn push(diags: &mut Vec<Diagnostic>, level: Level, span: Span, message: String) {
    diags.push(Diagnostic { level, message, primary: span });
}
