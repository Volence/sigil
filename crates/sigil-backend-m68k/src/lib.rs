//! 68000 `Backend` implementation: binds the CPU-agnostic `sigil_ir::Backend`
//! trait to `sigil_isa::m68k` and turns instructions into `DataFragment`s —
//! fully-resolved forms via `lower_inst`, and deferred-target forms via
//! `lower_branch` (bra/bsr/Bcc PcRel fixups), `lower_pcrel_ea` ((d16,PC) fixup),
//! and the two jmp/jsr forms. The assembler front-end selects jmp/jsr operand
//! WIDTH (abs.w vs abs.l) in its own pass loop and builds the finished fragment
//! via `lower_jmp_jsr_abs` (M1.D T3). `lower_jmp_jsr_sym` is the residual
//! placeholder form for hand-built IR, whose width the linker's `resolve_layout`
//! selects.

use sigil_ir::backend::{Backend, Cpu, LowerError};
use sigil_ir::{AbsWidth, DataFragment, Expr, Fixup, FixupKind, Fragment, RelaxCandidate};
use sigil_isa::m68k::{Instruction, Mnemonic, Operand, Size};
use sigil_span::Span;

/// Re-export the m68k vocabulary so downstream crates (the AS front-end) can
/// construct instructions without a *direct* `sigil-isa` dependency.
pub use sigil_isa::m68k;

/// The 68000 backend. Stateless.
pub struct M68kBackend;

impl Backend for M68kBackend {
    type Mnemonic = Mnemonic;
    type Operand = Operand;

    fn cpu(&self) -> Cpu {
        Cpu::M68000
    }

    /// The 68000 needs a size that the current `Backend::lower` signature does not
    /// carry (Z80 never did). Rather than mutate the shared trait in M1.A — which
    /// would ripple into `sigil-backend-z80` and the front-end — the trait method
    /// assumes **word** size (the correct default for the size-less mnemonics and
    /// the common case) and the **size-carrying tested path is `lower_inst`**.
    /// Whether to add a `size` param to the trait is a later (front-end) decision.
    /// Callers needing `.b`/`.l`/`.s` MUST use `lower_inst`.
    fn lower(&self, mnemonic: Mnemonic, operands: &[Operand], span: Span) -> Result<DataFragment, LowerError> {
        let inst = Instruction { mnemonic, size: sigil_isa::m68k::Size::W, ops: operands.to_vec() };
        self.lower_inst(&inst, span)
    }
}

impl M68kBackend {
    /// Lower a fully-formed `Instruction` (size already chosen) to a fragment.
    /// This is the primary, size-explicit adapter path (see the `lower` trait doc).
    pub fn lower_inst(&self, inst: &Instruction, span: Span) -> Result<DataFragment, LowerError> {
        let bytes = m68k::encode(inst).map_err(|e| LowerError { message: e.to_string() })?;
        Ok(DataFragment { bytes, fixups: vec![], span })
    }

    /// Lower a bare-symbol `jmp`/`jsr` to the length-variable placeholder the
    /// linker's `resolve_layout` will width-select and lower. `is_jsr` picks
    /// `jsr` (true) vs `jmp` (false).
    pub fn lower_jmp_jsr_sym(&self, is_jsr: bool, target: Expr, span: Span) -> Fragment {
        Fragment::JmpJsrSym { is_jsr, target, span }
    }

    /// Lower a bare-symbol `jmp`/`jsr` at an ALREADY-CHOSEN width to a finished
    /// `DataFragment` (opcode word + `Abs16Be`/`Abs32Be` fixup carrying `target`).
    ///
    /// This is the front-end's width-selected path (M1.D T3): the front-end folds
    /// the target and picks `width` via `asl_width_rule` in its own pass loop, so
    /// the fragment's byte length is final and the cursor advances truthfully — no
    /// deferral to `resolve_layout`. Byte layout matches the linker's private
    /// `lower_jmp_jsr` (jmp 4EF8/4EF9, jsr 4EB8/4EB9; `.l = .w | 1`; operand at
    /// offset 2). The value is still resolved by `link()`'s fixup pass.
    pub fn lower_jmp_jsr_abs(
        &self,
        is_jsr: bool,
        target: Expr,
        width: AbsWidth,
        span: Span,
    ) -> DataFragment {
        let base: u16 = if is_jsr { 0x4EB8 } else { 0x4EF8 };
        match width {
            AbsWidth::W => DataFragment {
                bytes: vec![(base >> 8) as u8, (base & 0xFF) as u8, 0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs16Be, offset: 2, target }],
                span,
            },
            AbsWidth::L => {
                let op = base | 0x0001;
                DataFragment {
                    bytes: vec![(op >> 8) as u8, (op & 0xFF) as u8, 0, 0, 0, 0],
                    fixups: vec![Fixup { kind: FixupKind::Abs32Be, offset: 2, target }],
                    span,
                }
            }
        }
    }

    /// Lower a symbolic `bra`/`bsr`/`Bcc` at an explicit size (`.s` or `.w`) to
    /// the opcode word + placeholder displacement + a PC-relative fixup. Aeon
    /// pins branch sizes, so the size is always known here (never selected).
    pub fn lower_branch(
        &self,
        mnemonic: Mnemonic,
        size: Size,
        target: Expr,
        span: Span,
    ) -> Result<DataFragment, LowerError> {
        let disp_op = match size {
            Size::S | Size::W => Operand::Disp(0),
            other => return Err(LowerError { message: format!("branch size {other:?} illegal on 68000") }),
        };
        let inst = Instruction { mnemonic, size, ops: vec![disp_op] };
        let encoded = m68k::encode(&inst).map_err(|e| LowerError { message: e.to_string() })?;
        match size {
            Size::S => {
                if encoded.len() != 2 {
                    return Err(LowerError { message: format!("bra.s expected 2 bytes, got {}", encoded.len()) });
                }
                Ok(DataFragment {
                    bytes: vec![encoded[0], 0x00],
                    fixups: vec![Fixup { kind: FixupKind::PcRel8, offset: 1, target }],
                    span,
                })
            }
            Size::W => {
                if encoded.len() != 4 {
                    return Err(LowerError { message: format!("bra.w expected 4 bytes, got {}", encoded.len()) });
                }
                Ok(DataFragment {
                    bytes: vec![encoded[0], encoded[1], 0x00, 0x00],
                    fixups: vec![Fixup { kind: FixupKind::PcRelDisp16, offset: 2, target }],
                    span,
                })
            }
            _ => unreachable!(),
        }
    }

    /// Build the four ordered candidate encodings for an emp `jbra`/`jbsr`
    /// auto-reaching branch (D2.18), smallest → largest, for a
    /// [`Fragment::RelaxLadder`] the linker width-selects:
    ///
    /// | rung | form          | bytes                     | fixup                    |
    /// |------|---------------|---------------------------|--------------------------|
    /// | 0    | `bra.s`/`bsr.s` | `60/61 00`              | `PcRel8` @ 1 (2 bytes)   |
    /// | 1    | `bra.w`/`bsr.w` | `60/61 00 00 00`        | `PcRelDisp16` @ 2 (4 b)  |
    /// | 2    | `jmp`/`jsr` abs.w | `4EF8/4EB8 00 00`     | `Abs16Be` @ 2 (4 bytes)  |
    /// | 3    | `jmp`/`jsr` abs.l | `4EF9/4EB9 00 00 00 00` | `Abs32Be` @ 2 (6 b)  |
    ///
    /// `is_jsr` selects the call opcodes (`bsr`/`jsr`) over the jump ones. The
    /// displacement/address bytes are zeroed placeholders the linker patches via
    /// each candidate's fixup. Rung 1 (`bra.w`, PC-relative) is ordered BEFORE
    /// rung 2 (`jmp abs.w`) though both are 4 bytes: same length, but the
    /// PC-relative form is relocatable and is the D2.18 ladder's preference. This
    /// mirrors the [`lower_jmp_jsr_abs`](Self::lower_jmp_jsr_abs)/`lower_branch`
    /// precedent — the m68k instruction encodings live in this backend, not in the
    /// emp front-end, which merely supplies the assembled candidates to the ladder.
    pub fn lower_jbra_jbsr_candidates(&self, is_jsr: bool, target: Expr, _span: Span) -> Vec<RelaxCandidate> {
        // bra/bsr opcode high byte: 0x60 (bra, cc=0) or 0x61 (bsr, cc=1).
        let br: u8 = if is_jsr { 0x61 } else { 0x60 };
        // jmp/jsr abs.w opcode word: 0x4EF8 (jmp) or 0x4EB8 (jsr); abs.l = `| 1`.
        let jmp_w: u16 = if is_jsr { 0x4EB8 } else { 0x4EF8 };
        let jmp_l: u16 = jmp_w | 0x0001;
        vec![
            // Rung 0 — bra.s/bsr.s: opcode byte + disp byte (offset 1), 2 bytes.
            RelaxCandidate {
                bytes: vec![br, 0x00],
                fixup: Fixup { kind: FixupKind::PcRel8, offset: 1, target: target.clone() },
            },
            // Rung 1 — bra.w/bsr.w: opcode word + disp word (offset 2), 4 bytes.
            RelaxCandidate {
                bytes: vec![br, 0x00, 0x00, 0x00],
                fixup: Fixup { kind: FixupKind::PcRelDisp16, offset: 2, target: target.clone() },
            },
            // Rung 2 — jmp/jsr abs.w: opcode word + abs.w address (offset 2), 4 bytes.
            RelaxCandidate {
                bytes: vec![(jmp_w >> 8) as u8, (jmp_w & 0xFF) as u8, 0x00, 0x00],
                fixup: Fixup { kind: FixupKind::Abs16Be, offset: 2, target: target.clone() },
            },
            // Rung 3 — jmp/jsr abs.l: opcode word + abs.l address (offset 2), 6 bytes.
            RelaxCandidate {
                bytes: vec![(jmp_l >> 8) as u8, (jmp_l & 0xFF) as u8, 0x00, 0x00, 0x00, 0x00],
                fixup: Fixup { kind: FixupKind::Abs32Be, offset: 2, target },
            },
        ]
    }

    /// Build the TWO ordered candidate encodings for an UNSIZED `bra`/`bsr`/`Bcc`
    /// (§5.4) — the `.s`→`.w` relaxation ladder Core width-selects. Unlike
    /// `jbra`/`jbsr`, an unsized conditional has NO far form (a `jmp`/`jsr`
    /// fallback is unconditional-only), so the ladder stops at `.w`: out of ±32K
    /// reach is Core's convergence error, not a wider rung.
    ///
    /// | rung | form   | bytes                | fixup                   |
    /// |------|--------|----------------------|-------------------------|
    /// | 0    | `.s`   | `<op> 00`            | `PcRel8` @ 1 (2 bytes)  |
    /// | 1    | `.w`   | `<op> 00 00 00`      | `PcRelDisp16` @ 2 (4 b) |
    ///
    /// The opcode is DERIVED from [`lower_branch`](Self::lower_branch) at each size
    /// — the SAME encoder path the sized `.s`/`.w` pins take — so the `cc` table is
    /// not duplicated here and the two rungs are byte-identical to the pinned forms
    /// the linker would emit for `<mnemonic>.s` / `<mnemonic>.w`. `mnemonic` is the
    /// already-classified `Bra`/`Bsr`/`Bcc(cond)`; a non-branch mnemonic is a
    /// caller bug and surfaces as the `lower_branch` size-illegal error.
    pub fn lower_unsized_branch_candidates(
        &self,
        mnemonic: Mnemonic,
        target: Expr,
        span: Span,
    ) -> Result<Vec<RelaxCandidate>, LowerError> {
        // Reuse the sized encoders so the ladder's bytes equal the `.s`/`.w` pins
        // exactly (opcode word derived once, in one place — no cc-table copy).
        let short = self.lower_branch(mnemonic, Size::S, target.clone(), span)?;
        let word = self.lower_branch(mnemonic, Size::W, target, span)?;
        // `lower_branch` emits EXACTLY one PC-relative fixup for `.s`/`.w`, so
        // `next()` is total; a missing fixup would be a `lower_branch` bug, not a
        // caller error — surface it as an internal LowerError rather than panic.
        let short_fx = short
            .fixups
            .into_iter()
            .next()
            .ok_or_else(|| LowerError { message: "internal: bra.s emitted no fixup".into() })?;
        let word_fx = word
            .fixups
            .into_iter()
            .next()
            .ok_or_else(|| LowerError { message: "internal: bra.w emitted no fixup".into() })?;
        Ok(vec![
            RelaxCandidate { bytes: short.bytes, fixup: short_fx },
            RelaxCandidate { bytes: word.bytes, fixup: word_fx },
        ])
    }

    /// Lower a symbolic `dbcc`/`dbra` (`DBcc Dn,disp`) to the opcode word +
    /// placeholder displacement + a PC-relative fixup. `dbcc` is always word-sized
    /// (no size suffix — unlike `bra`/`Bcc` there is no `.s` form).
    pub fn lower_dbcc(
        &self,
        cond: sigil_isa::m68k::Cond,
        dn: u8,
        target: Expr,
        span: Span,
    ) -> Result<DataFragment, LowerError> {
        let inst = Instruction {
            mnemonic: Mnemonic::Dbcc(cond),
            size: Size::W,
            ops: vec![Operand::Dn(dn), Operand::Disp(0)],
        };
        let encoded = m68k::encode(&inst).map_err(|e| LowerError { message: e.to_string() })?;
        if encoded.len() != 4 {
            return Err(LowerError { message: format!("dbcc expected 4 bytes, got {}", encoded.len()) });
        }
        Ok(DataFragment {
            bytes: vec![encoded[0], encoded[1], 0x00, 0x00],
            fixups: vec![Fixup { kind: FixupKind::PcRelDisp16, offset: 2, target }],
            span,
        })
    }

    /// Lower an instruction carrying a symbolic `(d16,PC)` operand: encode with a
    /// `Pcd16(0)` placeholder, then attach a `PcRelDisp16` fixup at the byte
    /// offset of that extension word. `pcd16_offset` is that offset within the
    /// encoded bytes (the caller/front-end knows the operand layout).
    pub fn lower_pcrel_ea(
        &self,
        inst: &Instruction,
        pcd16_offset: u32,
        target: Expr,
        span: Span,
    ) -> Result<DataFragment, LowerError> {
        let bytes = m68k::encode(inst).map_err(|e| LowerError { message: e.to_string() })?;
        if pcd16_offset as usize + 2 > bytes.len() {
            return Err(LowerError { message: "pcd16 offset past instruction end".into() });
        }
        Ok(DataFragment {
            bytes,
            fixups: vec![Fixup { kind: FixupKind::PcRelDisp16, offset: pcd16_offset, target }],
            span,
        })
    }

    /// Lower an instruction carrying a symbolic `(d8,PC,Xn)` operand: encode with
    /// a `Pcd8Xn { d: 0, .. }` placeholder (the encoder emits the brief extension
    /// word with a zero displacement), then attach a `PcRelDisp8` fixup at the
    /// disp (low) byte of that extension word. `disp8_offset` is that byte's
    /// position within the encoded bytes — for a single-word opcode with the
    /// PC-indexed EA as its source it is `3` (opcode word + ext-word high byte).
    pub fn lower_pcrel_idx_ea(
        &self,
        inst: &Instruction,
        disp8_offset: u32,
        target: Expr,
        span: Span,
    ) -> Result<DataFragment, LowerError> {
        let bytes = m68k::encode(inst).map_err(|e| LowerError { message: e.to_string() })?;
        if disp8_offset as usize >= bytes.len() {
            return Err(LowerError { message: "pcd8 disp offset past instruction end".into() });
        }
        Ok(DataFragment {
            bytes,
            fixups: vec![Fixup { kind: FixupKind::PcRelDisp8, offset: disp8_offset, target }],
            span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_isa::m68k::{Cond, Size};
    use sigil_span::SourceId;

    fn span() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    #[test]
    fn lowers_resolved_instruction_via_encode() {
        let b = M68kBackend;
        // move.w d1,d0 → 30 01
        let inst = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Operand::Dn(1), Operand::Dn(0)] };
        let frag = b.lower_inst(&inst, span()).unwrap();
        assert_eq!(frag.bytes, vec![0x30, 0x01]);
        assert!(frag.fixups.is_empty());
    }

    #[test]
    fn unsupported_form_becomes_lower_error() {
        let b = M68kBackend;
        // move with immediate destination is illegal.
        let inst = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Operand::Dn(0), Operand::Imm(1)] };
        assert!(b.lower_inst(&inst, span()).is_err());
    }

    #[test]
    fn reexports_m68k_vocabulary() {
        use crate::m68k::{Cond, Mnemonic, Operand, Size};
        let _ = (Mnemonic::Bcc(Cond::Eq), Operand::Dn(0), Size::W);
    }

    #[test]
    fn cpu_is_m68000() {
        assert_eq!(M68kBackend.cpu(), Cpu::M68000);
    }

    #[test]
    fn lower_jmp_jsr_sym_builds_placeholder_fragment() {
        let f = M68kBackend.lower_jmp_jsr_sym(true, Expr::Sym("Sub".into()), span());
        match f {
            Fragment::JmpJsrSym { is_jsr, target, .. } => {
                assert!(is_jsr);
                assert_eq!(target, Expr::Sym("Sub".into()));
            }
            _ => panic!("expected JmpJsrSym"),
        }
    }

    #[test]
    fn lower_branch_short_emits_opcode_plus_pcrel8() {
        let frag = M68kBackend
            .lower_branch(Mnemonic::Bra, Size::S, Expr::Sym(".t".into()), span())
            .unwrap();
        assert_eq!(frag.bytes, vec![0x60, 0x00]);
        assert_eq!(frag.fixups.len(), 1);
        assert_eq!(frag.fixups[0].kind, FixupKind::PcRel8);
        assert_eq!(frag.fixups[0].offset, 1);
    }

    #[test]
    fn lower_branch_word_emits_opcode_plus_pcreldisp16() {
        let frag = M68kBackend
            .lower_branch(Mnemonic::Bra, Size::W, Expr::Sym(".t".into()), span())
            .unwrap();
        assert_eq!(frag.bytes, vec![0x60, 0x00, 0x00, 0x00]);
        assert_eq!(frag.fixups[0].kind, FixupKind::PcRelDisp16);
        assert_eq!(frag.fixups[0].offset, 2);
    }

    #[test]
    fn lower_branch_bcc_uses_condition_opcode() {
        let frag = M68kBackend
            .lower_branch(Mnemonic::Bcc(Cond::Eq), Size::W, Expr::Sym(".t".into()), span())
            .unwrap();
        assert_eq!(&frag.bytes[..2], &[0x67, 0x00]);
    }

    #[test]
    fn lower_pcrel_ea_attaches_disp16_fixup_at_offset() {
        // lea (d16,PC),a0 → 41 FA 00 00 : opcode word, then the d16 extension word.
        let inst = Instruction { mnemonic: Mnemonic::Lea, size: Size::L, ops: vec![Operand::Pcd16(0), Operand::An(0)] };
        let frag = M68kBackend.lower_pcrel_ea(&inst, 2, Expr::Sym("L".into()), span()).unwrap();
        assert_eq!(frag.bytes.len(), 4);
        assert_eq!(frag.fixups.len(), 1);
        assert_eq!(frag.fixups[0].kind, FixupKind::PcRelDisp16);
        assert_eq!(frag.fixups[0].offset, 2);
        assert_eq!(frag.fixups[0].target, Expr::Sym("L".into()));
    }

    #[test]
    fn lower_pcrel_ea_offset_past_end_errors() {
        let inst = Instruction { mnemonic: Mnemonic::Lea, size: Size::L, ops: vec![Operand::Pcd16(0), Operand::An(0)] };
        // Encoded length is 4; offset 3 makes offset+2 = 5 > 4.
        let err = M68kBackend.lower_pcrel_ea(&inst, 3, Expr::Sym("L".into()), span()).unwrap_err();
        assert!(err.message.contains("past instruction end"));
    }

    #[test]
    fn lower_dbcc_emits_opcode_plus_pcreldisp16() {
        // dbf d0,* → 0x51C8, then placeholder displacement word + PcRelDisp16 fixup.
        let frag = M68kBackend
            .lower_dbcc(Cond::F, 0u8, Expr::Sym("loop".into()), span())
            .unwrap();
        assert_eq!(frag.bytes, vec![0x51, 0xC8, 0x00, 0x00]);
        assert_eq!(frag.fixups.len(), 1);
        assert_eq!(frag.fixups[0].kind, FixupKind::PcRelDisp16);
        assert_eq!(frag.fixups[0].offset, 2);
        assert_eq!(frag.fixups[0].target, Expr::Sym("loop".into()));
    }

    #[test]
    fn jbra_candidates_are_the_four_ordered_rungs() {
        // jbra: bra.s → bra.w → jmp abs.w → jmp abs.l, non-decreasing lengths.
        let c = M68kBackend.lower_jbra_jbsr_candidates(false, Expr::Sym("L".into()), span());
        assert_eq!(c.len(), 4);
        // Rung 0: bra.s = 60 00, PcRel8 @ 1 (2 bytes).
        assert_eq!(c[0].bytes, vec![0x60, 0x00]);
        assert_eq!(c[0].fixup.kind, FixupKind::PcRel8);
        assert_eq!(c[0].fixup.offset, 1);
        // Rung 1: bra.w = 60 00 00 00, PcRelDisp16 @ 2 (4 bytes).
        assert_eq!(c[1].bytes, vec![0x60, 0x00, 0x00, 0x00]);
        assert_eq!(c[1].fixup.kind, FixupKind::PcRelDisp16);
        assert_eq!(c[1].fixup.offset, 2);
        // Rung 2: jmp abs.w = 4E F8 00 00, Abs16Be @ 2 (4 bytes) — ranked AFTER
        // bra.w though same length (PC-relative preferred, D2.18).
        assert_eq!(c[2].bytes, vec![0x4E, 0xF8, 0x00, 0x00]);
        assert_eq!(c[2].fixup.kind, FixupKind::Abs16Be);
        assert_eq!(c[2].fixup.offset, 2);
        // Rung 3: jmp abs.l = 4E F9 00 00 00 00, Abs32Be @ 2 (6 bytes).
        assert_eq!(c[3].bytes, vec![0x4E, 0xF9, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(c[3].fixup.kind, FixupKind::Abs32Be);
        assert_eq!(c[3].fixup.offset, 2);
        // Lengths non-decreasing (the RelaxLadder construction contract).
        assert!(c.windows(2).all(|w| w[0].bytes.len() <= w[1].bytes.len()));
    }

    #[test]
    fn jbsr_candidates_use_bsr_and_jsr_opcodes() {
        // jbsr: bsr.s (61) → bsr.w (61) → jsr abs.w (4EB8) → jsr abs.l (4EB9).
        let c = M68kBackend.lower_jbra_jbsr_candidates(true, Expr::Sym("L".into()), span());
        assert_eq!(c[0].bytes, vec![0x61, 0x00]);
        assert_eq!(c[1].bytes, vec![0x61, 0x00, 0x00, 0x00]);
        assert_eq!(c[2].bytes, vec![0x4E, 0xB8, 0x00, 0x00]);
        assert_eq!(c[3].bytes, vec![0x4E, 0xB9, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn unsized_branch_candidates_are_two_pcrel_rungs() {
        // An unsized `bra` relaxes over exactly two rungs: bra.s (PcRel8 @1, 2B)
        // then bra.w (PcRelDisp16 @2, 4B). No far form (conditional/uniform §5.4).
        let c = M68kBackend
            .lower_unsized_branch_candidates(Mnemonic::Bra, Expr::Sym("L".into()), span())
            .unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].bytes, vec![0x60, 0x00]);
        assert_eq!(c[0].fixup.kind, FixupKind::PcRel8);
        assert_eq!(c[0].fixup.offset, 1);
        assert_eq!(c[1].bytes, vec![0x60, 0x00, 0x00, 0x00]);
        assert_eq!(c[1].fixup.kind, FixupKind::PcRelDisp16);
        assert_eq!(c[1].fixup.offset, 2);
        // Non-decreasing lengths (the RelaxLadder construction contract).
        assert!(c[0].bytes.len() <= c[1].bytes.len());
    }

    #[test]
    fn unsized_branch_candidates_use_condition_opcode() {
        // `bne` (cc=6) → opcode high byte 0x66, derived from the SAME cc table the
        // sized `bne.s`/`bne.w` pins use (via lower_branch) — not duplicated.
        let c = M68kBackend
            .lower_unsized_branch_candidates(Mnemonic::Bcc(Cond::Ne), Expr::Sym("L".into()), span())
            .unwrap();
        assert_eq!(&c[0].bytes[..1], &[0x66]);
        assert_eq!(&c[1].bytes[..1], &[0x66]);
        // `bhi` (cc=2) → 0x62.
        let h = M68kBackend
            .lower_unsized_branch_candidates(Mnemonic::Bcc(Cond::Hi), Expr::Sym("L".into()), span())
            .unwrap();
        assert_eq!(&h[0].bytes[..1], &[0x62]);
    }

    #[test]
    fn lower_jmp_jsr_abs_builds_absw_and_absl() {
        use sigil_ir::{AbsWidth, FixupKind};
        let w = M68kBackend.lower_jmp_jsr_abs(false, Expr::Sym("T".into()), AbsWidth::W, span());
        assert_eq!(w.bytes, vec![0x4E, 0xF8, 0x00, 0x00]);
        assert_eq!(w.fixups.len(), 1);
        assert!(matches!(w.fixups[0].kind, FixupKind::Abs16Be));
        assert_eq!(w.fixups[0].offset, 2);

        let l = M68kBackend.lower_jmp_jsr_abs(true, Expr::Sym("T".into()), AbsWidth::L, span());
        assert_eq!(l.bytes, vec![0x4E, 0xB9, 0x00, 0x00, 0x00, 0x00]);
        assert!(matches!(l.fixups[0].kind, FixupKind::Abs32Be));
        assert_eq!(l.fixups[0].offset, 2);
    }
}
