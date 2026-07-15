//! Lower a resolved [`CodeBuf`] to Core IR (Spec 2, Plan 4 — T3, D-P4.1/D-P4.2).
//! This is the backend-facing half of `asm { }`: the eval side (`eval/asm.rs`)
//! produced a CPU-neutral, splice-resolved [`Value::Code`]; here each
//! [`CodeItem`] becomes a label / backend instruction / inline data and is
//! streamed into an [`IrBuilder`]. Single-pass, defer-to-link placement
//! (D-P4.2): a branch/jmp/jsr target stays a symbolic [`Fixup`] the linker
//! resolves; a SIZED branch pins its width, an UNSIZED one defers width to
//! Core's relaxation ladder (§5.4). No `phys_base` bookkeeping.
//!
//! The operand + dispatch construction MIRRORS the AS front-end
//! (`sigil-frontend-as/src/eval.rs`): mnemonic-string → backend `Mnemonic`,
//! [`CodeOperand`] → backend `Operand`, then the same routing —
//! `bra`/`bsr`/`Bcc` → [`M68kBackend::lower_branch`], bare `jmp`/`jsr` to a
//! symbol → [`M68kBackend::lower_jmp_jsr_sym`] (deferred), everything else →
//! [`M68kBackend::lower_inst`]. An UNSIZED branch relaxes its `.s`/`.w` via Core
//! (§5.4); under `@as_compat` it stays the `[branch.missing-size]` error (a
//! faithful AS port pins every branch width).
//!
//! 68k is complete; Z80 is wired STRUCTURALLY (dispatch routes to
//! [`Z80Backend::lower`] / [`Z80Backend::lower_rel`]) but thin — the emp
//! operand-class model ([`Reg`] = `d0`..`a7`) is 68k-only, so Z80 register /
//! immediate operands aren't representable yet (a T1 model extension); those
//! forms diagnose rather than mis-encode.

use crate::value::{CodeBuf, CodeItem, CodeOperand, Reg, Width};
use sigil_backend_m68k::m68k::{
    Cond as M68kCond, Instruction as M68kInst, Mnemonic as M68kMnemonic, Operand as M68kOperand,
    Size as M68kSize, Xn as M68kXn,
};
use sigil_backend_m68k::M68kBackend;
use sigil_backend_z80::z80::{Mnemonic as Z80Mnemonic, Operand as Z80Operand};
use sigil_backend_z80::Z80Backend;
use sigil_ir::backend::{Backend, Cpu, IrStreamer};
use sigil_ir::{DataFragment, Expr, Fixup, FixupKind, Fragment, IrBuilder, RelaxCandidate};
use sigil_span::{Diagnostic, Level, Span};

/// Lower every item of `code` into the currently-open section of `builder`,
/// encoding for `cpu`. Diagnostics (unrecognized mnemonic, missing branch size,
/// unsupported operand form, encoder error) are appended to `diags`; a failing
/// item is skipped so one bad line does not abort the fragment. A standalone fn
/// so a `lower_code` test AND T4's proc lowering can both drive it.
///
/// `as_compat` is the enclosing module's `@as_compat` flag (D-P6.3): under it an
/// UNSIZED `bra`/`bsr`/`Bcc` keeps the `[branch.missing-size]` error (a faithful
/// AS port pins every branch width); without it an unsized branch relaxes its
/// `.s`/`.w` via Core (§5.4). It is the ONLY branch-lowering decision this flag
/// steers — explicit `.s`/`.w` pins and `jbra`/`jbsr` behave identically either way.
pub fn lower_code_buf(
    code: &CodeBuf,
    cpu: Cpu,
    as_compat: bool,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    for item in &code.items {
        match item {
            CodeItem::Label { name, .. } => builder.define_label(name),
            CodeItem::Inline(buf, span) => {
                // A `Data` value spliced into the code stream (§6.2) — today a
                // `dc.b`/`dc.w`/`dc.l` statement's cells (tranche 8). Scalars
                // serialize in the section CPU's byte order (68k BE / Z80 LE).
                let (bytes, fixups, mut ds) = super::data::stream_data(buf, cpu, *span);
                diags.append(&mut ds);
                builder.emit_data(&bytes, fixups, *span);
            }
            CodeItem::Instr { mnemonic, size, ops, span } => {
                // `jbra`/`jbsr` are emp-ONLY mnemonic-position words (D2.18): they
                // must NOT enter sigil-isa's shared mnemonic table (the AS
                // front-end keeps rejecting them). Recognize them HERE, before the
                // per-CPU isa dispatch, so the 68k-only guard reads uniformly for
                // both CPUs. A Z80 auto-reaching branch (`jr`→`jp` ladder) is
                // deferred, so on Z80 they are `[branch.non-68k]`.
                if matches!(mnemonic.as_str(), "jbra" | "jbsr") {
                    lower_jbra_jbsr(mnemonic, *size, ops, *span, cpu, builder, diags);
                    continue;
                }
                match cpu {
                    Cpu::M68000 => {
                        lower_m68k_instr(mnemonic, *size, ops, *span, as_compat, builder, diags)
                    }
                    Cpu::Z80 => lower_z80_instr(mnemonic, *size, ops, *span, builder, diags),
                }
            }
        }
    }
}

/// Whether `base` is a recognized MNEMONIC-position word for `cpu` (D-PP.1) —
/// i.e. a real CPU mnemonic OR the emp-only auto-reaching branches `jbra`/`jbsr`
/// (which the eval-side dispatch keeps out of the shared isa table, T2/D2.18).
/// This is the discriminator a bare statement call consults: mnemonics WIN
/// unconditionally (tenet 3), so a leading bareword that IS a recognized
/// mnemonic is an instruction, never a comptime-fn call — a comptime fn named
/// like a mnemonic (`move`, `jbra`) is simply unreachable at statement position.
/// `base` is the RAW leading word (no size suffix); the per-CPU recognizers
/// (`m68k_mnemonic`/`z80_mnemonic`) already fold the conditional-branch /
/// `dbcc`/`scc` families, so this stays a thin membership query over them.
pub(crate) fn is_recognized_mnemonic(base: &str, cpu: Cpu) -> bool {
    // `dc` is the code-embedded-data statement (tranche 8), CPU-neutral like
    // the DataBuf it produces — reserved on both CPUs so a comptime fn named
    // `dc` can never shadow it (tenet 3, same footing as jbra/jbsr).
    if matches!(base, "jbra" | "jbsr" | "dc") {
        return true;
    }
    match cpu {
        Cpu::M68000 => m68k_mnemonic(base).is_some(),
        Cpu::Z80 => z80_mnemonic(base).is_some(),
    }
}

// ---- 68000 -------------------------------------------------------------

/// Lower one 68k instruction, routing branches / bare jmp-jsr / generic exactly
/// as the AS front-end does.
fn lower_m68k_instr(
    mnemonic: &str,
    size: Option<Width>,
    ops: &[CodeOperand],
    span: Span,
    as_compat: bool,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let Some(m) = m68k_mnemonic(mnemonic) else {
        push_err(diags, span, format!("`{mnemonic}` is not a recognized 68000 mnemonic"));
        return;
    };

    // Control transfer whose target is resolved by the linker (D-P4.2).
    if matches!(m, M68kMnemonic::Bra | M68kMnemonic::Bsr | M68kMnemonic::Bcc(_)) {
        lower_m68k_branch(m, size, ops, span, as_compat, builder, diags);
        return;
    }
    if let M68kMnemonic::Dbcc(cond) = m {
        lower_m68k_dbcc(cond, ops, span, builder, diags);
        return;
    }
    if matches!(m, M68kMnemonic::Jmp | M68kMnemonic::Jsr) {
        // A bare symbol target defers to the linker's width selection; an EA
        // operand (`(a0)`, ...) falls through to the generic path. NOTE: a
        // `SymOff` (`jmp Item.field`) deliberately does NOT match this guard —
        // it falls through to the sym-count dispatch below and becomes an
        // absolute-address transfer via the abs-sym seam (see that site).
        if let [CodeOperand::Sym(name)] = ops {
            let frag = M68kBackend.lower_jmp_jsr_sym(
                matches!(m, M68kMnemonic::Jsr),
                Expr::Sym(name.clone()),
                span,
            );
            // abs.w baseline advance (4 bytes); the linker chooses the true width.
            builder.emit_fragment(frag, 4);
            return;
        }
    }
    if matches!(m, M68kMnemonic::Movem) {
        lower_m68k_movem(size, ops, span, builder, diags);
        return;
    }
    // A PC-relative operand (`Sym(pc)` / `Sym(pc,Xn.size)`) is an EXACT
    // (fixed-size) EA — no relaxation, unlike the RelaxAbsSym seam below. At
    // most one is legal per instruction (the 68k has one EA-operand slot that
    // can carry it; a second would be a malformed instruction the encoder
    // itself would reject), so route on presence, not count.
    if ops.iter().any(|o| matches!(o, CodeOperand::PcRel { .. } | CodeOperand::PcRelIdx { .. })) {
        lower_m68k_pcrel(m, size, ops, span, builder, diags);
        return;
    }

    // Generic fold-and-encode path.
    let size = match size {
        Some(w) => width_to_size(w),
        None => match m68k_default_size(m) {
            Some(s) => s,
            None => {
                push_err(diags, span, "instruction needs an explicit size suffix (.b/.w/.l)");
                return;
            }
        },
    };
    // A single symbolic absolute-address operand defers its width (abs.w/abs.l)
    // to the linker via a relaxable fragment (rather than the fixed generic path).
    // A `SymOff` (the D-PP.5 `Item.field` field-address form) is the SAME abs seam
    // — an absolute address whose fixup target is a `sym + off` sum — so it counts
    // and routes identically. This includes `jmp`/`jsr Item.field` (which the bare-
    // Sym guard above passes over): they become absolute-address transfers through
    // THIS seam (RelaxAbsSym, byte-pinned by `jmp_field_operand_is_absolute_
    // address_transfer`), not the `JmpJsrSym` linker ladder.
    // A link-time immediate routes to its own fixed-shape path (tranche 5 —
    // the emp mirror of the AS side's `try_defer_long_imm`).
    if ops.iter().any(|o| matches!(o, CodeOperand::ImmLink { .. })) {
        lower_m68k_imm_link(m, size, ops, span, builder, diags);
        return;
    }
    let sym_count = ops
        .iter()
        .filter(|o| {
            matches!(
                o,
                CodeOperand::Sym(_) | CodeOperand::SymOff { .. } | CodeOperand::AbsSym { .. }
            )
        })
        .count();
    match sym_count {
        0 => {}
        1 => {
            lower_m68k_abs_sym(m, size, ops, span, builder, diags);
            return;
        }
        _ => {
            // Two width-PINNED absolute operands (a mem-to-mem move, e.g.
            // `move.w (A).w, (B).w`) is a single finished encoding with two
            // fixed-width fixups — no relaxation (both widths are authored).
            // A bare (relaxable) operand among 2+ would need two-way RelaxAbsSym
            // (unbuilt) and still diagnoses.
            if let [
                CodeOperand::AbsSym { target: s, long: s_long },
                CodeOperand::AbsSym { target: d, long: d_long },
            ] = ops
            {
                lower_m68k_two_pinned_abs(
                    m, size, (s, *s_long), (d, *d_long), span, builder, diags,
                );
                return;
            }
            push_err(
                diags,
                span,
                "[lower.abs-sym-operand] two symbolic operands are supported only when both \
                 widths are pinned ((Sym).w/(Sym).l)",
            );
            return;
        }
    }

    let mut mops = Vec::with_capacity(ops.len());
    for op in ops {
        match m68k_operand(op) {
            Ok(o) => mops.push(o),
            Err(msg) => {
                push_err(diags, span, msg);
                return;
            }
        }
    }
    // asl zero-displacement optimization (`(0,An)` → `(An)`), mirroring the AS
    // front-end's post-conversion pass — `movep` is the sole 68000 exception
    // (it HAS no `(An)` mode; its d16 field is load-bearing even at 0). The
    // real demand site is typed field access on an offset-0 field
    // (`Sst.code_addr(a0)`, the object dispatch slot): asl emits the 4-byte
    // `(An)` form there, so byte parity requires the same collapse here.
    if m != M68kMnemonic::Movep {
        for op in &mut mops {
            collapse_zero_disp(op);
        }
    }
    let m = refine_m68k_mnemonic(m, &mops);
    let inst = M68kInst { mnemonic: m, size, ops: mops };
    match M68kBackend.lower_inst(&inst, span) {
        Ok(df) => emit_data_frag(builder, df),
        Err(e) => push_err(diags, span, e.message),
    }
}

/// `bra`/`bsr`/`Bcc <target>` (§5.4). Three cases, by size + `@as_compat`:
///
/// - **Explicit `.s`/`.w`** — a PIN, everywhere and unchanged: the single
///   symbolic target becomes a PC-relative fixup via [`M68kBackend::lower_branch`]
///   (byte-identical to before this task, `@as_compat` or not).
/// - **UNSIZED, non-`@as_compat`** — relax over the two-rung `.s`→`.w` ladder Core
///   width-selects ([`M68kBackend::lower_unsized_branch_candidates`]). There is no
///   far form: out of ±32K reach is Core's convergence error naming the distance
///   (an unsized `bra`/`bsr` that overshoots is steered toward `jbra`/`jbsr`).
/// - **UNSIZED, `@as_compat`** — the `[branch.missing-size]` error, VERBATIM: a
///   faithful AS port pins every branch width, so an unsized branch is a defect,
///   not a relaxation request.
///
/// The single label target is a hygiene-renamed [`CodeOperand::Sym`] — same
/// contract as the sized forms and `jbra`/`jbsr`.
fn lower_m68k_branch(
    m: M68kMnemonic,
    size: Option<Width>,
    ops: &[CodeOperand],
    span: Span,
    as_compat: bool,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    // Resolve the size FIRST so a bad width suffix / missing-size decision is made
    // before we touch operands (an unsized branch under `@as_compat` must error
    // with `[branch.missing-size]` regardless of the operand shape).
    let size = match size {
        Some(Width::S) => M68kSize::S,
        Some(Width::W) => M68kSize::W,
        Some(_) => {
            push_err(diags, span, "branch size suffix must be `.s` or `.w`");
            return;
        }
        None if as_compat => {
            // A faithful AS port pins branch widths (D-P6.3) — keep the pre-§5.4
            // error VERBATIM (same tag, same message) so ports stay unchanged.
            push_err(
                diags,
                span,
                "[branch.missing-size] branch needs an explicit size suffix (.s or .w) — \
                 Aeon pins branch width, no relaxation",
            );
            return;
        }
        None => {
            // §5.4: an unpinned branch is sized by Core's relaxation. Build the
            // `.s`→`.w` ladder and let `resolve_layout` pick the reaching rung
            // (out-of-reach becomes Core's convergence error, not a wider form).
            lower_unsized_branch(m, ops, span, builder, diags);
            return;
        }
    };
    let target = match ops {
        [CodeOperand::Sym(name)] => Expr::Sym(name.clone()),
        _ => {
            push_err(diags, span, "branch needs a single label target");
            return;
        }
    };
    match M68kBackend.lower_branch(m, size, target, span) {
        Ok(df) => emit_data_frag(builder, df),
        Err(e) => push_err(diags, span, e.message),
    }
}

/// An unsized `bra`/`bsr`/`Bcc` in a non-`@as_compat` module (§5.4): emit ONE
/// [`Fragment::RelaxLadder`] of two candidates (`.s`→`.w`) the linker
/// width-selects. Mirrors [`lower_jbra_jbsr`]'s ladder-emit shape — build the
/// candidates in the BACKEND, advance by the smallest (baseline `.s` = 2 bytes),
/// and let `resolve_layout` grow the fragment as the resolved target demands.
fn lower_unsized_branch(
    m: M68kMnemonic,
    ops: &[CodeOperand],
    span: Span,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let target = match ops {
        [CodeOperand::Sym(name)] => Expr::Sym(name.clone()),
        _ => {
            push_err(diags, span, "branch needs a single label target");
            return;
        }
    };
    let candidates = match M68kBackend.lower_unsized_branch_candidates(m, target.clone(), span) {
        Ok(c) => c,
        Err(e) => {
            push_err(diags, span, e.message);
            return;
        }
    };
    // Baseline advance = the smallest (first) candidate's length (`.s` = 2),
    // mirroring `jbra`/`RelaxAbsSym`; resolve_layout grows the fragment on demand.
    let advance = candidates[0].bytes.len() as u32;
    let frag = Fragment::RelaxLadder { candidates, target, span };
    builder.emit_fragment(frag, advance);
}

/// `jbra L` / `jbsr L`: emp-only auto-reaching branches (D2.18). Unlike sized
/// `bra`/`bsr` (which pin `.s`/`.w` and never relax), `jbra`/`jbsr` size THEMSELVES
/// — one [`Fragment::RelaxLadder`] with four ordered candidates the linker
/// width-selects: `bra.s → bra.w → jmp abs.w → jmp abs.l` (`jbsr`: `bsr`/`jsr`).
/// The baseline cursor advance is 2 (the smallest, `bra.s`, rung — mirroring how
/// `JmpJsrSym` advances by its abs.w baseline); the ladder grows the fragment
/// only as the resolved target demands. The candidate byte-blocks are built by
/// the m68k BACKEND ([`M68kBackend::lower_jbra_jbsr_candidates`]) so instruction
/// encodings stay out of this front-end file (the `RelaxAbsSym` precedent).
///
/// Diagnostics (all D2.18): a size suffix (`jbra.s`/`.w`/…) is `[jbra.sized]`
/// (jbra sizes itself); a non-label operand (`(a0)`, `#5`, two operands) is
/// `[jbra.label-only]` (naming `jmp`/`jsr` for computed targets); on a non-68k
/// (Z80) section it is `[branch.non-68k]` (the Z80 `jr → jp` ladder is deferred).
/// The single label target is a hygiene-renamed [`CodeOperand::Sym`] — the SAME
/// contract as sized branches and `jmp`/`jsr`, so a proc-local `.draw` target
/// (already qualified upstream by the hygiene pass) works identically.
fn lower_jbra_jbsr(
    mnemonic: &str,
    size: Option<Width>,
    ops: &[CodeOperand],
    span: Span,
    cpu: Cpu,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    // 68k-only: the Z80 auto-reaching-branch ladder is deferred (D2.18). Mirror
    // `[dispatch.non-68k]`'s guard shape with a branch-specific code.
    if cpu != Cpu::M68000 {
        push_err(
            diags,
            span,
            format!(
                "[branch.non-68k] `{mnemonic}` is a 68k auto-reaching branch; \
                 Z80 has no `jbra`/`jbsr` (a `jr`→`jp` ladder is not yet supported)"
            ),
        );
        return;
    }
    // `jbra` sizes ITSELF — a size suffix is a contradiction, not a pin. Steer to
    // the sized forms (`bra.s`/`bra.w`) or `jmp` for a computed target.
    if size.is_some() {
        push_err(
            diags,
            span,
            format!(
                "[jbra.sized] `{mnemonic}` sizes itself — drop the suffix \
                 (pin with bra.s/bra.w, or use jmp for computed targets)"
            ),
        );
        return;
    }
    // A single LABEL target only. A register-indirect / immediate / multi-operand
    // form is a computed transfer, which is `jmp`/`jsr`'s job, not `jbra`/`jbsr`'s.
    let target = match ops {
        [CodeOperand::Sym(name)] => Expr::Sym(name.clone()),
        _ => {
            push_err(
                diags,
                span,
                format!(
                    "[jbra.label-only] `{mnemonic}` needs a single label target; \
                     for a computed/register target use `jmp (a0)` / `jsr (a0)`"
                ),
            );
            return;
        }
    };
    let is_jsr = mnemonic == "jbsr";
    let candidates = M68kBackend.lower_jbra_jbsr_candidates(is_jsr, target.clone(), span);
    // Baseline advance = the smallest (first) candidate's length (bra.s = 2),
    // mirroring how JmpJsrSym advances by its abs.w baseline; resolve_layout grows
    // the fragment as the resolved target demands.
    let advance = candidates[0].bytes.len() as u32;
    let frag = Fragment::RelaxLadder { candidates, target, span };
    builder.emit_fragment(frag, advance);
}

/// An instruction carrying ONE `Sym(pc)` / `Sym(pc,Xn.size)` operand — a
/// PC-relative EA (68k `(d16,PC)` / `(d8,PC,Xn)`). Unlike the RelaxAbsSym
/// abs.w/abs.l seam, this is an EXACT (fixed-size) EA: `(d16,PC)` is always a
/// 2-byte extension word, `(d8,PC,Xn)` always a 2-byte brief extension word —
/// no relaxation, so ONE encoding via [`M68kBackend::lower_pcrel_ea`] /
/// [`M68kBackend::lower_pcrel_idx_ea`] with a placeholder-0 displacement and a
/// `PcRelDisp16`/`PcRelDisp8` fixup the linker resolves (same VMA-distance
/// arithmetic `bra`/`bsr`/`jbra` already use — cross-section-safe by
/// construction, see the `pcrel_port.rs` test module doc).
///
/// # Fixup offset
/// The 68k `encode_ea` rejects `Pcd16`/`Pcd8Xn` as a DESTINATION (PC-relative
/// only reads), so wherever it legally appears it is the SOURCE of a 2-operand
/// form or the single EA of a 1-operand form — both `encode_move`/
/// `encode_alu_ea`/etc. emit the source's extension words immediately after
/// the 2-byte opcode word (mirrors the AS front-end's `lower_m68k_pcrel`/
/// `lower_m68k_pcrel_idx` doc, which this offset convention is copied from).
/// So the plain form's d16 ext word always starts at byte offset 2; the
/// indexed form's brief ext word also starts at offset 2, and its disp8 is
/// that word's LOW byte, i.e. offset 3.
/// The link-time fixup expression for a PC-relative target: the bare symbol,
/// or `Sym ± n` when the operand carried a comptime addend (`Sym-4(pc,Xn)` —
/// the linker's `Expr::fold` does the arithmetic after symbol resolution).
fn pcrel_target_expr(target: &str, addend: i64) -> Expr {
    let sym = Expr::Sym(target.to_string());
    if addend == 0 {
        sym
    } else {
        Expr::Binary {
            op: sigil_ir::expr::BinOp::Add,
            lhs: Box::new(sym),
            rhs: Box::new(Expr::Int(addend)),
        }
    }
}

fn lower_m68k_pcrel(
    m: M68kMnemonic,
    size: Option<Width>,
    ops: &[CodeOperand],
    span: Span,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let size = match size {
        Some(w) => width_to_size(w),
        None => match m68k_default_size(m) {
            Some(s) => s,
            None => {
                push_err(diags, span, "instruction needs an explicit size suffix (.b/.w/.l)");
                return;
            }
        },
    };
    // Exactly one PC-relative operand is supported (the ISA has one EA slot
    // that can legally carry it); a second is a malformed instruction.
    let pcrel_count = ops
        .iter()
        .filter(|o| matches!(o, CodeOperand::PcRel { .. } | CodeOperand::PcRelIdx { .. }))
        .count();
    if pcrel_count > 1 {
        push_err(
            diags,
            span,
            "[lower.pcrel-operand] two PC-relative operands in one instruction is not \
             supported",
        );
        return;
    }
    let mut mops = Vec::with_capacity(ops.len());
    let mut target: Option<Expr> = None;
    let mut is_indexed = false;
    for op in ops {
        match op {
            CodeOperand::PcRel { target: t, addend } => {
                target = Some(pcrel_target_expr(t, *addend));
                mops.push(M68kOperand::Pcd16(0));
            }
            CodeOperand::PcRelIdx { target: t, addend, xn, xlong } => {
                target = Some(pcrel_target_expr(t, *addend));
                is_indexed = true;
                let (is_a, n) = reg_kind(*xn);
                let xn = if is_a { M68kXn::A(n) } else { M68kXn::D(n) };
                mops.push(M68kOperand::Pcd8Xn { d: 0, xn, long: *xlong });
            }
            other => match m68k_operand(other) {
                // movep has no pc-relative form, so the collapse is unconditional here.
                Ok(mut o) => {
                    collapse_zero_disp(&mut o);
                    mops.push(o);
                }
                Err(msg) => {
                    push_err(diags, span, msg);
                    return;
                }
            },
        }
    }
    let target = target.expect("caller guarantees exactly one PC-relative operand");
    let refined = refine_m68k_mnemonic(m, &mops);
    let inst = M68kInst { mnemonic: refined, size, ops: mops };
    let result = if is_indexed {
        M68kBackend.lower_pcrel_idx_ea(&inst, 3, target, span)
    } else {
        M68kBackend.lower_pcrel_ea(&inst, 2, target, span)
    };
    match result {
        Ok(df) => emit_data_frag(builder, df),
        Err(e) => push_err(diags, span, e.message),
    }
}

/// `dbcc`/`dbra Dn, <target>`: always word-sized (no size suffix — unlike
/// `bra`/`Bcc` there is no `.s` form, so a missing size is fine/expected here).
/// The two operands are a DATA register and a single label target; both become
/// a `dbf`/`db<cc>` opcode word + a PC-relative fixup via [`M68kBackend::lower_dbcc`].
fn lower_m68k_dbcc(
    cond: M68kCond,
    ops: &[CodeOperand],
    span: Span,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let (dn, name) = match ops {
        [CodeOperand::Reg(r), CodeOperand::Sym(name)] => match reg_kind(*r) {
            (false, n) => (n, name),
            (true, _) => {
                push_err(diags, span, "[dbcc.operands] dbcc needs `dN, <label>`");
                return;
            }
        },
        _ => {
            push_err(diags, span, "[dbcc.operands] dbcc needs `dN, <label>`");
            return;
        }
    };
    let target = Expr::Sym(name.clone());
    match M68kBackend.lower_dbcc(cond, dn, target, span) {
        Ok(df) => emit_data_frag(builder, df),
        Err(e) => push_err(diags, span, e.message),
    }
}

/// `movem.<w|l> <reglist>,<ea>` (STORE) / `movem.<w|l> <ea>,<reglist>` (LOAD).
/// The register-list operand already arrived as a resolved
/// [`CodeOperand::RegList`] (built by the eval-side `movem_reg_list`
/// recognizer, D-P1H.2) carrying the CANONICAL mask; this only validates size
/// and operand shape, maps the OTHER (memory-EA) operand through the ordinary
/// mapper, and hands both straight to the ISA encoder — direction (store vs
/// load) and the `-(An)` predecrement mask reversal are entirely the encoder's
/// job (mirrors the AS front-end's `lower_m68k_movem` doc comment). The
/// zero-displacement `(0,An)` → `(An)` collapse applies here as it does on
/// every other path (tranche 6 — asl collapses movem's memory EA too; the
/// earlier "out of scope" reading predated the offset-0 field-access class).
fn lower_m68k_movem(
    size: Option<Width>,
    ops: &[CodeOperand],
    span: Span,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let size = match size {
        Some(w @ (Width::W | Width::L)) => width_to_size(w),
        Some(_) => {
            push_err(diags, span, "movem is word (.w) or long (.l) only");
            return;
        }
        None => {
            push_err(diags, span, "movem needs an explicit size suffix (.w or .l)");
            return;
        }
    };
    let (mask, mem_op, list_first) = match ops {
        [CodeOperand::RegList(mask), mem] => (*mask, mem, true),
        [mem, CodeOperand::RegList(mask)] => (*mask, mem, false),
        _ => {
            push_err(
                diags,
                span,
                "movem needs two operands: a register list and a memory EA",
            );
            return;
        }
    };
    let mut mem_op = match m68k_operand(mem_op) {
        Ok(o) => o,
        Err(msg) => {
            push_err(diags, span, msg);
            return;
        }
    };
    collapse_zero_disp(&mut mem_op);
    let list_op = M68kOperand::RegList(mask);
    let ops = if list_first { vec![list_op, mem_op] } else { vec![mem_op, list_op] };
    let inst = M68kInst { mnemonic: M68kMnemonic::Movem, size, ops };
    match M68kBackend.lower_inst(&inst, span) {
        Ok(df) => emit_data_frag(builder, df),
        Err(e) => push_err(diags, span, e.message),
    }
}

/// A LINK-TIME immediate source (tranche 5 `.l`, tranche 6 `.w` — the emp
/// mirror of the AS front-end's `try_defer_long_imm`): `movea.l #SONG_TABLE,
/// a0` / `move.w #ROUTINE_OFF, (a0)` where the value is an extern()/equ
/// residual that cannot fold until link. The instruction encodes ONCE with a
/// zero-imm placeholder and ONE `Value32Be` (`.l`) / `Value16Be` (`.w`) fixup
/// at offset 2 (the imm field always directly follows the one-word opcode:
/// the imm is the SOURCE, so its ext words come first; any destination ext
/// words follow the hole and don't move it).
///
/// The `.w` width's demand site is the object-bank dispatch store (tranche 6:
/// `move.w #(Main - ObjCodeBase), Sst.code_addr(a0)` — objroutine): a
/// link-time symbol DIFFERENCE in a word immediate. `Value16Be` range-checks
/// the folded value to the unsigned 16-bit window at link, so a negative or
/// oversize difference is loud, not wrapped.
///
/// Fenced to the proven shapes: `.w`/`.l` sizes only (a `.b` symbolic imm has
/// no deferral yet — the remaining width of the ledgered extension gap,
/// consumer-gated), the imm FIRST, and no other symbolic operand (their
/// fixups would collide).
fn lower_m68k_imm_link(
    m: M68kMnemonic,
    size: M68kSize,
    ops: &[CodeOperand],
    span: Span,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    if !matches!(size, M68kSize::L | M68kSize::W) {
        push_err(
            diags,
            span,
            "[lower.imm-link] a link-time immediate needs `.w` or `.l` size — a `.b` symbolic \
             immediate has no deferral yet (mirror the value into a comptime const, or \
             extend the imm-deferral family)",
        );
        return;
    }
    if !matches!(ops.first(), Some(CodeOperand::ImmLink { .. })) {
        push_err(
            diags,
            span,
            "[lower.imm-link] a link-time immediate is only supported as the SOURCE \
             (first) operand",
        );
        return;
    }
    // Opcode-embedded-immediate families (quick forms, shift counts, moveq)
    // have no 32-bit imm field to defer into — steer BEFORE the backend sees
    // the zero placeholder (which would otherwise leak into its range error:
    // "Addq data must be 1..=8, got 0").
    if matches!(
        m,
        M68kMnemonic::Moveq
            | M68kMnemonic::Addq
            | M68kMnemonic::Subq
            | M68kMnemonic::Lsl
            | M68kMnemonic::Lsr
            | M68kMnemonic::Asl
            | M68kMnemonic::Asr
            | M68kMnemonic::Rol
            | M68kMnemonic::Ror
    ) {
        push_err(
            diags,
            span,
            "[lower.imm-link] this instruction embeds its immediate in the opcode word \
             (moveq/quick/shift-count forms) — no link-time deferral; mirror the value \
             into a comptime const instead",
        );
        return;
    }
    let mut target: Option<Expr> = None;
    // A single PINNED-absolute operand (`(Sym).w`/`.l`) is ADMITTED alongside the
    // imm (tranche 10 — core's Init/Alloc SP-write shape): its address is a
    // SECOND, independent fixup at a distinct offset. `Some((target, long))` when
    // seen. RELAXABLE absolutes (`Sym`/`SymOff`) stay refused below — their
    // abs.w/abs.l width selection would genuinely conflict with the imm deferral.
    let mut abs: Option<(Expr, bool)> = None;
    let mut enc_ops = Vec::with_capacity(ops.len());
    for op in ops {
        match op {
            CodeOperand::ImmLink { target: t } => {
                if target.replace(t.clone()).is_some() {
                    push_err(
                        diags,
                        span,
                        "[lower.imm-link] two link-time immediates in one instruction",
                    );
                    return;
                }
                enc_ops.push(M68kOperand::Imm(0));
            }
            CodeOperand::AbsSym { target: n, long } => {
                if abs.is_some() {
                    push_err(
                        diags,
                        span,
                        "[lower.imm-link] a link-time immediate combined with more than one \
                         symbolic absolute operand is not yet supported",
                    );
                    return;
                }
                abs = Some((Expr::Sym(n.clone()), *long));
                enc_ops.push(if *long {
                    M68kOperand::AbsL(0)
                } else {
                    M68kOperand::AbsW(0)
                });
            }
            CodeOperand::Sym(_) | CodeOperand::SymOff { .. } => {
                push_err(
                    diags,
                    span,
                    "[lower.imm-link] a link-time immediate combined with another symbolic \
                     operand is not yet supported",
                );
                return;
            }
            // Any OTHER extension-word operand AFTER a pinned-abs operand would
            // land its ext words BEHIND the abs field and move the abs fixup
            // offset (mirrors `lower_m68k_abs_sym`'s caution) — still deferred.
            other if operand_has_ext_words(other) && abs.is_some() => {
                push_err(
                    diags,
                    span,
                    "[lower.imm-link] a symbolic absolute operand followed by another \
                     extension-word operand is not yet supported",
                );
                return;
            }
            other => match m68k_operand(other) {
                // The zero-disp collapse applies to the non-imm operands here
                // too — the `.w` demand site's DESTINATION is exactly an
                // offset-0 field EA (`Sst.code_addr(a0)` → `(a0)`, asl's 30BC
                // shape). movep can't reach this path (its EA pairs with Dn,
                // never an immediate), so the collapse is unconditional.
                Ok(mut o) => {
                    collapse_zero_disp(&mut o);
                    enc_ops.push(o);
                }
                Err(msg) => {
                    push_err(diags, span, msg);
                    return;
                }
            },
        }
    }
    let target = target.expect("guarded above: ops[0] is the ImmLink");
    let refined = refine_m68k_mnemonic(m, &enc_ops);
    let inst = M68kInst { mnemonic: refined, size, ops: enc_ops };
    let df = match M68kBackend.lower_inst(&inst, span) {
        Ok(df) => df,
        Err(e) => {
            push_err(diags, span, e.message);
            return;
        }
    };
    // Defense-in-depth: one opcode word + the imm field (4 bytes for `.l`,
    // 2 for `.w`) — anything shorter means the immediate landed in the opcode
    // word itself and there is no hole to defer into.
    // A `.w` link-time immediate is a WORD IMMEDIATE, not a data value or an
    // EA address: it holds the low 16 bits of a value whose high half must be a
    // consistent extension — all-zero (an objroutine offset in `[0, 0xFFFF]`)
    // or all-one (a sign-extended RAM address like `$FFFF9EDE`, core's
    // free-stack SP writes). `ImmWord16Be` is exactly that union — AS's
    // word-immediate rule. `Value16Be` rejects the sign-extended address;
    // `Abs16Be` rejects the `[0x8000, 0xFFFF]` upper-unsigned half (a valid
    // objroutine offset). `.l` stays a verbatim `Value32Be` (a full 32-bit
    // address fits without truncation).
    let (kind, min_len) = match size {
        M68kSize::L => (FixupKind::Value32Be, 6),
        M68kSize::W => (FixupKind::ImmWord16Be, 4),
        _ => unreachable!("guarded at entry: imm-link size is .w or .l"),
    };
    if df.bytes.len() < min_len {
        push_err(
            diags,
            span,
            "[lower.imm-link] this instruction embeds its immediate in the opcode word \
             (moveq/quick/shift-count forms) — no link-time deferral; mirror the value \
             into a comptime const instead",
        );
        return;
    }
    let mut df = df;
    df.fixups.push(Fixup { kind, offset: 2, target });
    // The pinned-absolute destination is a SECOND, independent fixup at a
    // distinct offset. The imm is the SOURCE, so ITS ext words come first —
    // the abs ext word follows the opcode word (2) + the imm field
    // (`imm_field_width`). Byte-exact for core's `move.w #imm, (abs).w` (imm
    // @2, abs.w @4) and its `.l`/abs.l relatives.
    if let Some((abs_target, long)) = abs {
        let imm_field_width = match size {
            M68kSize::W => 2,
            M68kSize::L => 4,
            _ => unreachable!("guarded at entry: imm-link size is .w or .l"),
        };
        let abs_kind = if long { FixupKind::Abs32Be } else { FixupKind::Abs16Be };
        df.fixups.push(Fixup { kind: abs_kind, offset: 2 + imm_field_width, target: abs_target });
    }
    emit_data_frag(builder, df);
}

/// Lower a straight-line instruction whose TWO operands are both width-PINNED
/// symbolic absolutes (`(Sym).w`/`(Sym).l`) — a memory-to-memory move, e.g.
/// section.asm's `move.w (Cache_Top_Row).w, (Section_Top_Row_Written).w`.
///
/// Both widths are authored, so there is ONE finished encoding (no RelaxAbsSym
/// candidate pair): encode with zeroed abs placeholders, then place two
/// fixed-width fixups. 68k extension-word order is source-then-destination, so
/// the source fixup sits right after the opcode word (offset 2) and the
/// destination fixup after the source's ext field (offset `2 + src_width`).
/// `move` takes exactly two operands and neither carries an extra ext word, so
/// no other operand can shift these offsets.
fn lower_m68k_two_pinned_abs(
    m: M68kMnemonic,
    size: M68kSize,
    src: (&str, bool),
    dst: (&str, bool),
    span: Span,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let (src_target, src_long) = src;
    let (dst_target, dst_long) = dst;
    let abs_op = |long: bool| if long { M68kOperand::AbsL(0) } else { M68kOperand::AbsW(0) };
    let enc_ops = vec![abs_op(src_long), abs_op(dst_long)];
    let refined = refine_m68k_mnemonic(m, &enc_ops);
    let inst = M68kInst { mnemonic: refined, size, ops: enc_ops };
    let mut df = match M68kBackend.lower_inst(&inst, span) {
        Ok(df) => df,
        Err(e) => {
            push_err(diags, span, e.message);
            return;
        }
    };
    let abs_kind = |long: bool| if long { FixupKind::Abs32Be } else { FixupKind::Abs16Be };
    let src_width = if src_long { 4 } else { 2 };
    df.fixups.push(Fixup {
        kind: abs_kind(src_long),
        offset: 2,
        target: Expr::Sym(src_target.to_string()),
    });
    df.fixups.push(Fixup {
        kind: abs_kind(dst_long),
        offset: 2 + src_width,
        target: Expr::Sym(dst_target.to_string()),
    });
    emit_data_frag(builder, df);
}

/// Lower a straight-line instruction carrying exactly ONE symbolic
/// absolute-address operand to a length-variable [`Fragment::RelaxAbsSym`]: the
/// front-end encodes BOTH the `abs.w` and `abs.l` candidates (with a zeroed
/// address placeholder), and the linker's `resolve_layout` selects one by the
/// resolved target address (§5.6 `asl_width_rule`).
///
/// # Fixup offset (the byte-exactness crux)
/// The symbolic operand's abs extension words must be the LAST bytes of the
/// encoding, so the fixup offset is exactly `short_bytes.len() - 2` (==
/// `long_bytes.len() - 4`) — identical in both candidates, differing only in
/// extension WIDTH (2 vs 4) and fixup KIND (`Abs16Be` vs `Abs32Be`).
///
/// What GUARANTEES that placement is POSITIONAL (tranche 5, relaxed from the
/// first cut's every-other-operand-ext-free rule): an extension-word operand
/// (`#imm`, `(d16,An)`) is allowed only at a position BEFORE the sym operand
/// (`move.w #$0100, (Z80_BUS_REQUEST).l` — the stopZ80 shape: the 68k emits
/// ext words in operand order, source first, so a preceding imm's words land
/// BEFORE the abs words and shift `offset` by a constant both candidates
/// share). One AFTER the sym operand would land ext words BEHIND the abs
/// field (`move.w (Sym).l, (d16,An)`) and stays deferred with the clear
/// diagnostic. Everything that could break "ops order == ext-word order"
/// (movem's mask word, branches, pc-rel) is routed away before this fn. The
/// pre-emission length check below is NOT that proof — it is
/// defense-in-depth against a broken backend / unexpected multi-word opcode.
fn lower_m68k_abs_sym(
    m: M68kMnemonic,
    size: M68kSize,
    ops: &[CodeOperand],
    span: Span,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    // Build the two operand lists — identical except the symbolic operand is
    // `AbsW(0)` in one and `AbsL(0)` in the other. Any OTHER operand must be
    // extension-word-free so the abs operand's ext words stay last (offset exact).
    let mut short_ops = Vec::with_capacity(ops.len());
    let mut long_ops = Vec::with_capacity(ops.len());
    // The abs operand's fixup target: a bare `Sym` for `move.w Foo, d0`, or a
    // `sym + off` sum for the D-PP.5 `Item.field` field-address form. Captured
    // as the Core `Expr` so the linker folds it (rename canonicalizes the inner
    // `Sym`; `asl_width_rule` widths the folded SUM).
    let mut target: Option<Expr> = None;
    // `Some(width)` when the author PINNED the width (`(Sym).w`/`(Sym).l`,
    // tranche 3): emit ONE finished candidate with a fixed-width fixup
    // instead of the RelaxAbsSym pair.
    let mut pinned: Option<bool> = None;
    for op in ops {
        match op {
            CodeOperand::Sym(n) => {
                target = Some(Expr::Sym(n.clone()));
                short_ops.push(M68kOperand::AbsW(0));
                long_ops.push(M68kOperand::AbsL(0));
            }
            CodeOperand::AbsSym { target: n, long } => {
                target = Some(Expr::Sym(n.clone()));
                pinned = Some(*long);
                short_ops.push(M68kOperand::AbsW(0));
                long_ops.push(M68kOperand::AbsL(0));
            }
            CodeOperand::SymOff { sym, off } => {
                target = Some(Expr::Binary {
                    op: sigil_ir::expr::BinOp::Add,
                    lhs: Box::new(Expr::Sym(sym.clone())),
                    // The field offset is a small non-negative struct offset;
                    // Core `Expr::Int` is `i64`, so narrow the `i128` offset.
                    rhs: Box::new(Expr::Int(*off as i64)),
                });
                short_ops.push(M68kOperand::AbsW(0));
                long_ops.push(M68kOperand::AbsL(0));
            }
            other if operand_has_ext_words(other) && target.is_some() => {
                // Ext words AFTER the sym operand would land BEHIND the abs
                // field and move the fixup offset — still deferred. (BEFORE
                // it they precede the abs field, which stays last — allowed
                // since tranche 5.)
                push_err(
                    diags,
                    span,
                    "[lower.abs-sym-operand] a symbolic absolute operand followed by another \
                     extension-word operand (#imm / (d16,An)) is not yet supported",
                );
                return;
            }
            other => match m68k_operand(other) {
                // movep never carries an abs-sym operand (Dn ↔ d16(An) only),
                // so the zero-disp collapse is unconditional on this path.
                Ok(mut o) => {
                    collapse_zero_disp(&mut o);
                    short_ops.push(o);
                    long_ops.push(o);
                }
                Err(msg) => {
                    push_err(diags, span, msg);
                    return;
                }
            },
        }
    }
    let target = target.expect("caller guarantees exactly one symbolic operand");

    // Refine against the abs.w operands; the choice is width-agnostic here (both
    // `AbsW` and `AbsL` are memory-EA destinations, so `refine` picks the same
    // mnemonic for either candidate).
    let refined = refine_m68k_mnemonic(m, &short_ops);
    let short_inst = M68kInst { mnemonic: refined, size, ops: short_ops };
    let long_inst = M68kInst { mnemonic: refined, size, ops: long_ops };
    let short_bytes = match M68kBackend.lower_inst(&short_inst, span) {
        Ok(df) => df.bytes,
        Err(e) => {
            push_err(diags, span, e.message);
            return;
        }
    };
    let long_bytes = match M68kBackend.lower_inst(&long_inst, span) {
        Ok(df) => df.bytes,
        Err(e) => {
            push_err(diags, span, e.message);
            return;
        }
    };
    // Defense-in-depth (NOT the offset-placement proof — that rests on
    // `operand_has_ext_words` completeness + the emp `CodeOperand` model having no
    // absolute/indexed/PC-relative EA but `Sym`, so `Sym` is the only ext-word
    // producer; see the fn doc). This guard only checks the two candidates differ
    // by exactly the abs.w→abs.l width delta (2 bytes) atop a ≥1-word opcode —
    // catching a broken backend or an unexpected multi-word opcode before we place
    // a fixup. A candidate pair that satisfied it while the abs ext were NOT last
    // would still be mis-offset, which is why the real guarantee lives upstream.
    if short_bytes.len() < 4 || long_bytes.len() != short_bytes.len() + 2 {
        push_err(
            diags,
            span,
            "[lower.abs-sym-operand] internal: abs.w/abs.l candidate widths are inconsistent",
        );
        return;
    }
    let offset = (short_bytes.len() - 2) as u32;
    // Author-pinned width: no relaxation — one finished encoding, one
    // fixed-width fixup (the same `offset` works for both: the long
    // candidate is the short plus 2 ext bytes, so `short.len()-2` ==
    // `long.len()-4`, i.e. the ext field's start either way).
    if let Some(long) = pinned {
        let (bytes, kind) = if long {
            (long_bytes, FixupKind::Abs32Be)
        } else {
            (short_bytes, FixupKind::Abs16Be)
        };
        let df = DataFragment { bytes, fixups: vec![Fixup { kind, offset, target }], span };
        emit_data_frag(builder, df);
        return;
    }
    let advance = short_bytes.len() as u32; // baseline (abs.w) cursor advance
    let frag = Fragment::RelaxAbsSym {
        short: RelaxCandidate {
            bytes: short_bytes,
            fixup: Fixup { kind: FixupKind::Abs16Be, offset, target: target.clone() },
        },
        long: RelaxCandidate {
            bytes: long_bytes,
            fixup: Fixup { kind: FixupKind::Abs32Be, offset, target: target.clone() },
        },
        target,
        span,
    };
    builder.emit_fragment(frag, advance);
}

/// True for a [`CodeOperand`] that contributes its own extension word(s) to the
/// encoding (`#imm`, `(d16,An)`). These make the abs operand's ext offset
/// position-dependent, so they are out of scope for the first symbolic-operand
/// cut. Register / `(An)` / `-(An)` / `(An)+` forms are ext-free. A `Sym` is the
/// symbolic operand itself (handled by the caller) and a `Cc` is never a valid
/// EA — neither is classified here.
fn operand_has_ext_words(op: &CodeOperand) -> bool {
    matches!(
        op,
        CodeOperand::Imm(_)
            | CodeOperand::DispInd { .. }
            | CodeOperand::IndIdx { .. }
            | CodeOperand::AbsInt { .. }
    )
}

/// Map a [`CodeOperand`] to a 68k [`M68kOperand`]. A symbolic operand only makes
/// sense in this position as a branch/jmp/jsr target (handled before this), so a
/// `Sym` here is an unsupported straight-line form.
///
/// This is the byte-exactness seam every path (asm eval AND, later, T4 proc
/// lowering) crosses, so the width-narrowing casts (`i128` → `i32` immediate /
/// `i16` displacement) are RANGE-CHECKED here rather than truncating silently —
/// mirroring the AS front-end's `m68k_imm_bounds` / `fold_imm(disp, i16..)`.
/// asl's zero-displacement optimization: a `(d16,An)` EA whose displacement
/// resolved to 0 encodes as plain `(An)` (2 bytes + 1 memory cycle cheaper).
/// Twin of the AS front-end's `collapse_zero_disp`; every m68k lowering path
/// that can carry a `Disp16An` applies it (gated off only for `movep`, whose
/// encoding has no `(An)` mode). The demand class is typed field access on an
/// offset-0 struct field — `Sst.code_addr(a0)`, the object dispatch slot.
fn collapse_zero_disp(op: &mut M68kOperand) {
    if let M68kOperand::Disp16An(0, n) = *op {
        *op = M68kOperand::Ind(n);
    }
}

fn m68k_operand(op: &CodeOperand) -> Result<M68kOperand, String> {
    match op {
        CodeOperand::ImmLink { .. } => {
            // Routed by `lower_m68k_imm_link` before the generic path; reaching
            // here means an unsupported combination (e.g. a Z80 instruction, or
            // a non-first position the router rejected).
            Err("a link-time immediate is only supported as a 68k `.w`/`.l` source operand".into())
        }
        CodeOperand::Imm(n) => {
            let n = *n;
            // A 32-bit immediate field admits either the signed or the
            // bit-pattern-equivalent unsigned spelling (i32::MIN..=u32::MAX); the
            // encoder does the size-specific business-rule check. Beyond that
            // range the value cannot be represented — diagnose, don't wrap.
            if n < i32::MIN as i128 || n > u32::MAX as i128 {
                return Err(format!("immediate out of range for a 32-bit operand: {n}"));
            }
            Ok(M68kOperand::Imm(n as i32))
        }
        CodeOperand::Reg(r) => Ok(m68k_reg_operand(*r)),
        CodeOperand::Sr => Ok(M68kOperand::Sr),
        CodeOperand::Ccr => Ok(M68kOperand::Ccr),
        CodeOperand::Ind(r) => {
            an_index(*r).map(M68kOperand::Ind).ok_or_else(|| ind_reg_err(*r))
        }
        CodeOperand::PreDec(r) => {
            an_index(*r).map(M68kOperand::PreDec).ok_or_else(|| ind_reg_err(*r))
        }
        CodeOperand::PostInc(r) => {
            an_index(*r).map(M68kOperand::PostInc).ok_or_else(|| ind_reg_err(*r))
        }
        CodeOperand::DispInd { disp, reg } => {
            let an = an_index(*reg).ok_or_else(|| ind_reg_err(*reg))?;
            let d = *disp;
            // The (d16,An) displacement is a signed 16-bit field: a value outside
            // it would silently wrap to a wrong offset, so diagnose instead.
            if d < i16::MIN as i128 || d > i16::MAX as i128 {
                return Err(format!("displacement out of range for (d16,An): {d}"));
            }
            Ok(M68kOperand::Disp16An(d as i16, an))
        }
        CodeOperand::AbsInt { addr, long } => {
            let a = *addr;
            // Eval range-checked already; re-check as defense-in-depth at
            // the byte-exactness seam, mirroring DispInd/IndIdx.
            if *long {
                if a < i32::MIN as i128 || a > u32::MAX as i128 {
                    return Err(format!("address out of range for abs.l: {a}"));
                }
                Ok(M68kOperand::AbsL(a as u32 as i32))
            } else {
                let w = (a as i64) & 0xFF_FFFF;
                if !(w <= 0x7FFF || w >= 0xFF_8000) {
                    return Err(format!("address {a:#X} has no abs.w spelling"));
                }
                Ok(M68kOperand::AbsW((a as i64 & 0xFFFF) as u16 as i16))
            }
        }
        // A pinned symbolic abs is always routed through `lower_m68k_abs_sym`
        // by the sym-count dispatch, exactly like `Sym`/`SymOff` — defense.
        CodeOperand::AbsSym { target, .. } => Err(format!(
            "symbolic absolute operand `{target}` in this position is not yet supported              (routed via the abs-sym seam)"
        )),
        CodeOperand::IndIdx { reg, disp, xn, xlong } => {
            let an = an_index(*reg).ok_or_else(|| ind_reg_err(*reg))?;
            let d = *disp;
            // Eval range-checked this already (`map_an_indexed`); re-check as
            // defense-in-depth at the byte-exactness seam, mirroring DispInd.
            if d < i8::MIN as i128 || d > i8::MAX as i128 {
                return Err(format!("displacement out of range for (d8,An,Xn): {d}"));
            }
            let (is_a, n) = reg_kind(*xn);
            let xn = if is_a { M68kXn::A(n) } else { M68kXn::D(n) };
            Ok(M68kOperand::Disp8AnXn { d: d as i8, an, xn, long: *xlong })
        }
        CodeOperand::Cc(_) => {
            Err("a condition code is not valid as an instruction operand".to_string())
        }
        CodeOperand::Sym(name) => Err(format!(
            "symbolic operand `{name}` in a straight-line instruction is not yet supported \
             (only branch / jmp / jsr targets defer to the linker)"
        )),
        // A `SymOff` (D-PP.5 field-address operand) is always routed through
        // `lower_m68k_abs_sym` by the sym-count dispatch, exactly like `Sym`, so
        // it never reaches this generic per-operand mapper.
        CodeOperand::SymOff { sym, off } => Err(format!(
            "field-address operand `{sym} + {off}` in a straight-line instruction is not yet \
             supported (routed via the abs-sym relaxation seam)"
        )),
        // A `RegList` is produced ONLY by the `movem` reglist recognizer
        // (D-P1H.2, eval/asm.rs `movem_reg_list`) and consumed directly by
        // `lower_m68k_movem` below — it never reaches this generic per-operand
        // mapper. Defense-in-depth, not a reachable path.
        CodeOperand::RegList(_) => {
            Err("internal: a movem register list reached the generic operand mapper".to_string())
        }
        // A PC-relative operand is always routed through `lower_m68k_pcrel` by
        // the mnemonic dispatch (it's detected before this generic mapper
        // runs), so it never reaches here. Defense-in-depth, not a reachable
        // path — mirrors the `RegList` arm above.
        CodeOperand::PcRel { .. } | CodeOperand::PcRelIdx { .. } => Err(
            "internal: a PC-relative operand reached the generic operand mapper".to_string(),
        ),
    }
}

/// A data/address register to its 68k register operand.
fn m68k_reg_operand(r: Reg) -> M68kOperand {
    match reg_kind(r) {
        (false, n) => M68kOperand::Dn(n),
        (true, n) => M68kOperand::An(n),
    }
}

/// The low-3-bit register number of an ADDRESS register (`a0`..`a7`), or `None`
/// for a data register (illegal as an indirect base).
fn an_index(r: Reg) -> Option<u8> {
    match reg_kind(r) {
        (true, n) => Some(n),
        (false, _) => None,
    }
}

fn ind_reg_err(r: Reg) -> String {
    format!("indirect base must be an address register (a0-a7), got {r}")
}

/// `(is_address_register, low-3-bit number)` for a [`Reg`].
pub(super) fn reg_kind(r: Reg) -> (bool, u8) {
    match r {
        Reg::D0 => (false, 0),
        Reg::D1 => (false, 1),
        Reg::D2 => (false, 2),
        Reg::D3 => (false, 3),
        Reg::D4 => (false, 4),
        Reg::D5 => (false, 5),
        Reg::D6 => (false, 6),
        Reg::D7 => (false, 7),
        Reg::A0 => (true, 0),
        Reg::A1 => (true, 1),
        Reg::A2 => (true, 2),
        Reg::A3 => (true, 3),
        Reg::A4 => (true, 4),
        Reg::A5 => (true, 5),
        Reg::A6 => (true, 6),
        Reg::A7 => (true, 7),
    }
}

/// The emp [`Width`] to a 68k [`M68kSize`].
fn width_to_size(w: Width) -> M68kSize {
    match w {
        Width::B => M68kSize::B,
        Width::W => M68kSize::W,
        Width::L => M68kSize::L,
        Width::S => M68kSize::S,
    }
}

/// A 68k mnemonic string (already lowercased, size-stripped) to its
/// [`M68kMnemonic`], including the `b<cc>`/`db<cc>`/`s<cc>` conditional families.
/// Mirrors the AS front-end's `m68k_mnemonic`/`m68k_cond`.
fn m68k_mnemonic(base: &str) -> Option<M68kMnemonic> {
    use M68kMnemonic::*;
    Some(match base {
        "move" => Move,
        "movea" => Movea,
        "add" => Add,
        "adda" => Adda,
        "sub" => Sub,
        "suba" => Suba,
        "and" => And,
        "or" => Or,
        "eor" => Eor,
        "cmp" => Cmp,
        "cmpa" => Cmpa,
        "muls" => Muls,
        "mulu" => Mulu,
        "addi" => Addi,
        "subi" => Subi,
        "andi" => Andi,
        "ori" => Ori,
        "eori" => Eori,
        "cmpi" => Cmpi,
        "moveq" => Moveq,
        "addq" => Addq,
        "subq" => Subq,
        "asl" => Asl,
        "asr" => Asr,
        "lsl" => Lsl,
        "lsr" => Lsr,
        "rol" => Rol,
        "ror" => Ror,
        "btst" => Btst,
        "bset" => Bset,
        "bclr" => Bclr,
        "clr" => Clr,
        "neg" => Neg,
        "not" => Not,
        "tst" => Tst,
        "tas" => Tas,
        "swap" => Swap,
        "ext" => Ext,
        "lea" => Lea,
        "pea" => Pea,
        "movem" => Movem,
        "movep" => Movep,
        "addx" => Addx,
        "cmpm" => Cmpm,
        "nop" => Nop,
        "rts" => Rts,
        "rte" => Rte,
        "trap" => Trap,
        "illegal" => Illegal,
        "bra" => Bra,
        "bsr" => Bsr,
        "jmp" => Jmp,
        "jsr" => Jsr,
        "dbf" | "dbra" => Dbcc(M68kCond::F),
        _ => {
            if let Some(rest) = base.strip_prefix("db") {
                if let Some(c) = m68k_cond(rest) {
                    return Some(Dbcc(c));
                }
            }
            if let Some(rest) = base.strip_prefix('b') {
                if let Some(c) = m68k_cond(rest) {
                    return Some(Bcc(c));
                }
            }
            if let Some(rest) = base.strip_prefix('s') {
                if let Some(c) = m68k_cond(rest) {
                    return Some(Scc(c));
                }
            }
            return None;
        }
    })
}

/// A 68k condition-code suffix to its [`M68kCond`] (all 16, plus the `hs`/`lo`
/// unsigned-branch spellings that alias `cc`/`cs`). Mirrors AS's `m68k_cond`.
fn m68k_cond(w: &str) -> Option<M68kCond> {
    use M68kCond::*;
    Some(match w {
        "t" => T,
        "f" => F,
        "hi" => Hi,
        "ls" => Ls,
        "cc" => Cc,
        "cs" => Cs,
        "hs" => Cc,
        "lo" => Cs,
        "ne" => Ne,
        "eq" => Eq,
        "vc" => Vc,
        "vs" => Vs,
        "pl" => Pl,
        "mi" => Mi,
        "ge" => Ge,
        "lt" => Lt,
        "gt" => Gt,
        "le" => Le,
        _ => return None,
    })
}

/// The implicit size for mnemonics that real 68k syntax never suffixes. Mirrors
/// the AS front-end's `m68k_default_size`; branches deliberately have NO default
/// (Aeon pins them explicitly).
fn m68k_default_size(m: M68kMnemonic) -> Option<M68kSize> {
    use M68kMnemonic::*;
    match m {
        Moveq => Some(M68kSize::L),
        Lea | Pea => Some(M68kSize::L),
        Swap | Nop | Rts | Rte | Tas | Trap | Illegal => Some(M68kSize::W),
        Jmp | Jsr => Some(M68kSize::W),
        Btst | Bset | Bclr => Some(M68kSize::B),
        Dbcc(_) => Some(M68kSize::W),
        Scc(_) => Some(M68kSize::B),
        _ => None,
    }
}

/// Refine a mnemonic against its resolved operands (asl's `move…sr` / `andi…ccr`
/// spellings and the `#imm,mem` → `xxxi` immediate forms). Mirrors AS's
/// `refine_m68k_mnemonic`.
fn refine_m68k_mnemonic(m: M68kMnemonic, ops: &[M68kOperand]) -> M68kMnemonic {
    use M68kMnemonic::*;
    match (m, ops) {
        (Move, [_, M68kOperand::Sr]) => MoveToSr,
        (Move, [M68kOperand::Sr, _]) => MoveFromSr,
        (Andi, [_, M68kOperand::Ccr]) => AndiCcr,
        (Ori, [_, M68kOperand::Ccr]) => OriCcr,
        (Cmp, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Cmpi,
        (Cmp, [_, M68kOperand::An(_)]) => Cmpa,
        (And, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Andi,
        (Or, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Ori,
        (Add, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Addi,
        (Sub, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Subi,
        (Eor, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Eori,
        (m, _) => m,
    }
}

/// True for a 68k MEMORY effective-address destination (routes `#imm,mem`
/// forms to their immediate encodings). Mirrors AS's `is_mem_dest`.
fn is_mem_dest(op: &M68kOperand) -> bool {
    use M68kOperand::*;
    matches!(
        op,
        Ind(_) | PostInc(_) | PreDec(_) | Disp16An(..) | Disp8AnXn { .. } | AbsW(_) | AbsL(_)
    )
}

// ---- Z80 (structural, thin — see module doc) ---------------------------

/// Lower one Z80 instruction. STRUCTURAL only: the emp operand-class model is
/// 68k-only, so only no-operand forms and symbolic `jr`/`djnz` are representable;
/// anything with register/immediate operands diagnoses (a T1 model extension is
/// needed before Z80 gains real operand depth).
fn lower_z80_instr(
    mnemonic: &str,
    _size: Option<Width>,
    ops: &[CodeOperand],
    span: Span,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let Some(m) = z80_mnemonic(mnemonic) else {
        push_err(diags, span, format!("`{mnemonic}` is not a recognized Z80 mnemonic"));
        return;
    };
    // Relative branch to a symbolic target → linker-resolved Z80JrRel8 fixup.
    if matches!(m, Z80Mnemonic::Jr | Z80Mnemonic::Djnz) {
        if let [CodeOperand::Sym(name)] = ops {
            match Z80Backend.lower_rel(m, None, Expr::Sym(name.clone()), span) {
                Ok(df) => emit_data_frag(builder, df),
                Err(e) => push_err(diags, span, e.message),
            }
            return;
        }
    }
    // No-operand fixed forms encode directly.
    if ops.is_empty() {
        let empty: [Z80Operand; 0] = [];
        match Z80Backend.lower(m, &empty, span) {
            Ok(df) => emit_data_frag(builder, df),
            Err(e) => push_err(diags, span, e.message),
        }
        return;
    }
    push_err(
        diags,
        span,
        format!(
            "[lower.z80-unsupported] Z80 operand form for `{mnemonic}` is not yet supported \
             (the emp operand-class model is 68k-only pending a T1 extension)"
        ),
    );
}

/// A small Z80 mnemonic table for the structurally-wired forms.
fn z80_mnemonic(base: &str) -> Option<Z80Mnemonic> {
    use Z80Mnemonic::*;
    Some(match base {
        "nop" => Nop,
        "ret" => Ret,
        "exx" => Exx,
        "rrca" => Rrca,
        "scf" => Scf,
        "ei" => Ei,
        "di" => Di,
        "ldir" => Ldir,
        "neg" => Neg,
        "jr" => Jr,
        "djnz" => Djnz,
        _ => return None,
    })
}

// ---- shared emit helpers ------------------------------------------------

/// Emit a finished [`DataFragment`] (bytes + fixups) into the open section.
fn emit_data_frag(builder: &mut IrBuilder, df: DataFragment) {
    builder.emit_data(&df.bytes, df.fixups, df.span);
}

/// Push an error diagnostic at `span`.
fn push_err(diags: &mut Vec<Diagnostic>, span: Span, message: impl Into<String>) {
    diags.push(Diagnostic { level: Level::Error, message: message.into(), primary: span });
}
