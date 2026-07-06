//! Lower a resolved [`CodeBuf`] to Core IR (Spec 2, Plan 4 — T3, D-P4.1/D-P4.2).
//! This is the backend-facing half of `asm { }`: the eval side (`eval/asm.rs`)
//! produced a CPU-neutral, splice-resolved [`Value::Code`]; here each
//! [`CodeItem`] becomes a label / backend instruction / inline data and is
//! streamed into an [`IrBuilder`]. Single-pass, defer-to-link placement
//! (D-P4.2): a branch/jmp/jsr target stays a symbolic [`Fixup`] the linker
//! resolves — no relaxation, no `phys_base` bookkeeping.
//!
//! The operand + dispatch construction MIRRORS the AS front-end
//! (`sigil-frontend-as/src/eval.rs`): mnemonic-string → backend `Mnemonic`,
//! [`CodeOperand`] → backend `Operand`, then the same routing —
//! `bra`/`bsr`/`Bcc` → [`M68kBackend::lower_branch`], bare `jmp`/`jsr` to a
//! symbol → [`M68kBackend::lower_jmp_jsr_sym`] (deferred), everything else →
//! [`M68kBackend::lower_inst`]. A bare (size-less) branch is the
//! `[branch.missing-size]` error (D-P4.2 — Aeon pins branch width).
//!
//! 68k is complete; Z80 is wired STRUCTURALLY (dispatch routes to
//! [`Z80Backend::lower`] / [`Z80Backend::lower_rel`]) but thin — the emp
//! operand-class model ([`Reg`] = `d0`..`a7`) is 68k-only, so Z80 register /
//! immediate operands aren't representable yet (a T1 model extension); those
//! forms diagnose rather than mis-encode.

use crate::value::{CodeBuf, CodeItem, CodeOperand, Reg, Width};
use sigil_backend_m68k::m68k::{
    Cond as M68kCond, Instruction as M68kInst, Mnemonic as M68kMnemonic, Operand as M68kOperand,
    Size as M68kSize,
};
use sigil_backend_m68k::M68kBackend;
use sigil_backend_z80::z80::{Mnemonic as Z80Mnemonic, Operand as Z80Operand};
use sigil_backend_z80::Z80Backend;
use sigil_ir::backend::{Backend, Cpu, IrStreamer};
use sigil_ir::{DataFragment, Expr, IrBuilder};
use sigil_span::{Diagnostic, Level, Span};

/// Lower every item of `code` into the currently-open section of `builder`,
/// encoding for `cpu`. Diagnostics (unrecognized mnemonic, missing branch size,
/// unsupported operand form, encoder error) are appended to `diags`; a failing
/// item is skipped so one bad line does not abort the fragment. A standalone fn
/// so a `lower_code` test AND T4's proc lowering can both drive it.
pub fn lower_code_buf(
    code: &CodeBuf,
    cpu: Cpu,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    for item in &code.items {
        match item {
            CodeItem::Label { name, .. } => builder.define_label(name),
            CodeItem::Inline(buf) => {
                // A `Data` value spliced into the code stream (§6.2). It carries
                // no span of its own; anchor any diagnostic at a zero span.
                // TODO(T4/T5): thread a real span once inline-data splices are
                // actually produced (unreachable until proc lowering exercises it).
                let span = Span { source: sigil_span::SourceId(0), start: 0, end: 0 };
                let (bytes, fixups, mut ds) = super::data::stream_data(buf, cpu, span);
                diags.append(&mut ds);
                builder.emit_data(&bytes, fixups, span);
            }
            CodeItem::Instr { mnemonic, size, ops, span } => match cpu {
                Cpu::M68000 => lower_m68k_instr(mnemonic, *size, ops, *span, builder, diags),
                Cpu::Z80 => lower_z80_instr(mnemonic, *size, ops, *span, builder, diags),
            },
        }
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
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let Some(m) = m68k_mnemonic(mnemonic) else {
        push_err(diags, span, format!("`{mnemonic}` is not a recognized 68000 mnemonic"));
        return;
    };

    // Control transfer whose target is resolved by the linker (D-P4.2).
    if matches!(m, M68kMnemonic::Bra | M68kMnemonic::Bsr | M68kMnemonic::Bcc(_)) {
        lower_m68k_branch(m, size, ops, span, builder, diags);
        return;
    }
    if let M68kMnemonic::Dbcc(cond) = m {
        lower_m68k_dbcc(cond, ops, span, builder, diags);
        return;
    }
    if matches!(m, M68kMnemonic::Jmp | M68kMnemonic::Jsr) {
        // A bare symbol target defers to the linker's width selection; an EA
        // operand (`(a0)`, ...) falls through to the generic path.
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
    let m = refine_m68k_mnemonic(m, &mops);
    let inst = M68kInst { mnemonic: m, size, ops: mops };
    match M68kBackend.lower_inst(&inst, span) {
        Ok(df) => emit_data_frag(builder, df),
        Err(e) => push_err(diags, span, e.message),
    }
}

/// `bra`/`bsr`/`Bcc <target>`: Aeon pins branch width (`.s`/`.w`, no relaxation),
/// so a missing size is the `[branch.missing-size]` error (D-P4.2). The single
/// symbolic target becomes a PC-relative fixup via [`M68kBackend::lower_branch`].
fn lower_m68k_branch(
    m: M68kMnemonic,
    size: Option<Width>,
    ops: &[CodeOperand],
    span: Span,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let size = match size {
        Some(Width::S) => M68kSize::S,
        Some(Width::W) => M68kSize::W,
        Some(_) => {
            push_err(diags, span, "branch size suffix must be `.s` or `.w`");
            return;
        }
        None => {
            push_err(
                diags,
                span,
                "[branch.missing-size] branch needs an explicit size suffix (.s or .w) — \
                 Aeon pins branch width, no relaxation",
            );
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

/// Map a [`CodeOperand`] to a 68k [`M68kOperand`]. A symbolic operand only makes
/// sense in this position as a branch/jmp/jsr target (handled before this), so a
/// `Sym` here is an unsupported straight-line form.
///
/// This is the byte-exactness seam every path (asm eval AND, later, T4 proc
/// lowering) crosses, so the width-narrowing casts (`i128` → `i32` immediate /
/// `i16` displacement) are RANGE-CHECKED here rather than truncating silently —
/// mirroring the AS front-end's `m68k_imm_bounds` / `fold_imm(disp, i16..)`.
fn m68k_operand(op: &CodeOperand) -> Result<M68kOperand, String> {
    match op {
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
        CodeOperand::Cc(_) => {
            Err("a condition code is not valid as an instruction operand".to_string())
        }
        CodeOperand::Sym(name) => Err(format!(
            "symbolic operand `{name}` in a straight-line instruction is not yet supported \
             (only branch / jmp / jsr targets defer to the linker)"
        )),
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
fn reg_kind(r: Reg) -> (bool, u8) {
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
        Swap | Nop | Rts | Rte | Tas | Trap => Some(M68kSize::W),
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
