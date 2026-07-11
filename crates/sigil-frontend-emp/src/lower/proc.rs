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
use crate::value::{CodeItem, CodeOperand};
use sigil_ir::backend::{Cpu, IrStreamer};
use sigil_ir::IrBuilder;
use sigil_span::{Diagnostic, Level, Span};
use std::collections::HashSet;

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
        if !writes_dest_register(mnemonic) {
            continue;
        }
        // An SR destination is a machine-state clobber (tranche 5): undeclared
        // unless the contract carries `clobbers(sr)`/`out(sr)` or `preserves(sr)`
        // (the latter's balance is checked separately).
        if matches!(ops.last(), Some(CodeOperand::Sr))
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
        // The destination is the last operand; only a register destination is a
        // clobber (a memory-dest form writes memory, not a register). Reuse
        // `Reg`'s `Display` for the canonical `d0`..`a7` spelling.
        let Some(CodeOperand::Reg(r)) = ops.last() else { continue };
        // Stack-pointer ARITHMETIC (`addq.l #2, sp` / `lea N(sp), sp`
        // cleanup) is stack DISCIPLINE, not a register clobber — every
        // push/pop-balancing proc adjusts sp, and balanced-stack
        // verification is S2-D7(b)'s dataflow job. Stack REPLACEMENT
        // (`movea.l x, sp` — switching stacks) stays a genuine a7 clobber
        // and is NOT exempt (tranche-3 review scoping).
        if *r == crate::value::Reg::A7 && is_sp_arithmetic(mnemonic, ops) {
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

/// The standard 68k write-form mnemonics whose LAST operand is the written
/// destination, plus the `s<cc>` family (`seq`/`sne`/`spl`/…, all `Scc` — they
/// set a byte in their sole operand). Read-only / control forms (`cmp`, `tst`,
/// `btst`, `bra`, `bsr`, `jmp`, `jsr`, `pea`, `nop`, `rts`…) are absent by
/// design so they never trip the lint.
///
/// KNOWN, DELIBERATE BLIND SPOT: `dbcc`/`dbra`/`dbf` decrement their FIRST
/// operand, not the last — the "destination is the last operand" model does not
/// hold for them, and encoding a per-mnemonic destination position is genuinely
/// S2-D6 territory, so they are intentionally NOT flagged here.
/// TODO(S2-D6): give the lint a per-mnemonic dest-position notion and cover dbcc.
///
/// This is a PARALLEL string list the compiler cannot keep honest against the
/// ISA `Mnemonic` set: a newly-supported write-form (`mulu`, `bchg`, `roxl`, …)
/// will silently escape the lint until it is added HERE. Keep this in sync as
/// the backend's mnemonic table grows. Heuristic (see [`check_clobbers`]).
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

    // Collect every stack movem (push to / pop from sp), with its size. The
    // rule: every stack movem whose list INTERSECTS the declared set must
    // EQUAL it (so a wrong-list early-exit restore is caught, while a
    // DISJOINT nested save around an inner call stays none of our business),
    // must be `movem.l` (a `.w` restore SIGN-EXTENDS each word — it does not
    // preserve anything), and at least one matching save must precede the
    // last matching restore.
    let mut matching_push: Option<usize> = None;
    let mut matching_pop: Option<usize> = None;
    for (i, item) in buf.items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, size, ops, span } = item else { continue };
        if mnemonic != "movem" {
            continue;
        }
        let (mask, is_push) = match ops.as_slice() {
            [CodeOperand::RegList(m), CodeOperand::PreDec(crate::value::Reg::A7)] => (*m, true),
            [CodeOperand::PostInc(crate::value::Reg::A7), CodeOperand::RegList(m)] => (*m, false),
            _ => continue,
        };
        if mask & declared == 0 {
            continue; // disjoint nested save — not part of this contract
        }
        if *size != Some(crate::value::Width::L) {
            push(
                diags,
                Level::Error,
                *span,
                format!(
                    "[proc.preserves-word-pair] `{}` declares `preserves({})` but this stack \
                     movem is not `movem.l` — a word-size restore sign-extends each register \
                     and preserves nothing; use `movem.l`",
                    proc.name,
                    mask_reglist(declared),
                ),
            );
            return;
        }
        if mask != declared {
            push(
                diags,
                Level::Error,
                *span,
                format!(
                    "[proc.preserves-mismatch] `{}` declares `preserves({})` but this stack \
                     movem {} `{}` — every save/restore overlapping the declared set must \
                     cover exactly that set; update the attribute or the movem",
                    proc.name,
                    mask_reglist(declared),
                    if is_push { "saves" } else { "restores" },
                    mask_reglist(mask),
                ),
            );
            return;
        }
        if is_push {
            if matching_push.is_none() {
                matching_push = Some(i);
            }
        } else {
            matching_pop = Some(i);
        }
    }
    let paired = matches!((matching_push, matching_pop), (Some(p), Some(q)) if p < q);
    if !paired {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.preserves-missing-pair] `{}` declares `preserves({})` but its body has \
                 no `movem.l <list>, -(sp)` … `movem.l (sp)+, <list>` save/restore pair — the \
                 syntactic slice (S2-D6b) verifies the literal movem pair, so a proc that \
                 preserves registers another way cannot declare `preserves` yet",
                proc.name,
                mask_reglist(declared),
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
    // claim. Same write detection as check_clobbers: the destination (last
    // operand) of a write-form mnemonic, when it is a register.
    let mut written: HashSet<String> = HashSet::new();
    for item in &buf.items {
        let CodeItem::Instr { mnemonic, ops, .. } = item else { continue };
        if !writes_dest_register(mnemonic) {
            continue;
        }
        if let Some(CodeOperand::Reg(r)) = ops.last() {
            written.insert(r.to_string());
        }
    }
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
