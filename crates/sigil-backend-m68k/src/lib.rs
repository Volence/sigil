//! 68000 `Backend` implementation: binds the CPU-agnostic `sigil_ir::Backend`
//! trait to `sigil_isa::m68k` and turns instructions into `DataFragment`s —
//! fully-resolved forms via `lower_inst`, and deferred-target forms via
//! `lower_branch` (bra/bsr/Bcc PcRel fixups), `lower_pcrel_ea` ((d16,PC) fixup),
//! and `lower_jmp_jsr_sym` (the jmp/jsr placeholder). Only jmp/jsr operand-WIDTH
//! selection (abs.w vs abs.l) is deferred to the linker's `resolve_layout`.

use sigil_ir::backend::{Backend, Cpu, LowerError};
use sigil_ir::{DataFragment, Expr, Fixup, FixupKind, Fragment};
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
}
