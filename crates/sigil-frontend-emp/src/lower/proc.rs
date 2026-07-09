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

    // 4. Clobbers lint (only when the proc declares a clobber set) — likewise a
    // modernization warning silenced under `@as_compat`.
    if !proc.clobbers.is_empty() && !ctx.as_compat {
        check_clobbers(proc, &buf, diags);
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
    let mut allowed: HashSet<&str> = proc.clobbers.iter().map(String::as_str).collect();
    // Params are declarative register bindings (§5.1): a write to a param
    // register is part of the proc's own contract, not an undeclared clobber.
    allowed.extend(proc.params.iter().map(|(name, _, _)| name.as_str()));

    for item in &buf.items {
        let CodeItem::Instr { mnemonic, ops, span, .. } = item else { continue };
        if !writes_dest_register(mnemonic) {
            continue;
        }
        // The destination is the last operand; only a register destination is a
        // clobber (a memory-dest form writes memory, not a register). Reuse
        // `Reg`'s `Display` for the canonical `d0`..`a7` spelling.
        let Some(CodeOperand::Reg(r)) = ops.last() else { continue };
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

/// Push a diagnostic at `span`.
fn push(diags: &mut Vec<Diagnostic>, level: Level, span: Span, message: String) {
    diags.push(Diagnostic { level, message, primary: span });
}
