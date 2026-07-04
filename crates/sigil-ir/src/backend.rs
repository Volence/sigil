//! CPU tag and (Task 4) the Backend / IrStreamer traits.

/// Which instruction set a [`crate::Section`]'s bytes are encoded for.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Cpu {
    Z80,
    M68000,
}

use crate::{DataFragment, Fixup, Fragment};
use sigil_span::{Diagnostic, Span};

/// An error produced while a backend lowers an instruction to bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LowerError {
    pub message: String,
}

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LowerError {}

/// A CPU-parameterized encoder. Generic over the CPU's `Mnemonic`/`Operand`
/// vocabulary so `sigil-ir` needs no dependency on any specific ISA crate
/// (the one-way crate-graph rule). `sigil-backend-z80` binds these to
/// `sigil_isa::z80::{Mnemonic, Operand}`.
///
/// M0 has no relaxation (every Z80 form is fixed-length), so lowering yields a
/// single [`DataFragment`] directly.
///
/// **Scope of this trait method:** `lower` handles instructions whose operands
/// are already fully resolved to concrete integers. Relative branches with an
/// *unresolved* target (`jr`/`djnz` to a symbol) are **out of scope for `lower`**
/// — it has no slot for a symbolic `Expr` target. Each backend exposes those via
/// a backend-specific inherent method (e.g. `Z80Backend::lower_rel`) that emits
/// the opcode plus a `Z80JrRel8` [`Fixup`] for the linker to resolve. A front-end
/// (Plan 4) must route relative-branch mnemonics to that path, not through `lower`.
pub trait Backend {
    type Mnemonic;
    type Operand;

    /// The CPU this backend encodes for.
    fn cpu(&self) -> Cpu;

    /// Lower one fully-resolved instruction to a data fragment. See the trait
    /// doc: `jr`/`djnz` with an unresolved symbolic target are NOT handled here.
    fn lower(
        &self,
        mnemonic: Self::Mnemonic,
        operands: &[Self::Operand],
        span: Span,
    ) -> Result<DataFragment, LowerError>;
}

/// The streaming sink a front-end emits into (M0 subset of Core §4.9). The
/// front-end folds every expression and lowers every instruction to final bytes
/// *before* streaming, so this trait carries only the doors that shape the
/// emitted `Module`: section boundaries, byte/fill/reserve fragments, labels,
/// diagnostics. The `save`/`restore` assembler-state stack lives in the
/// front-end, not here.
pub trait IrStreamer {
    /// Close the current section (if any) and open a new one. `vma_base = Some(v)`
    /// phases labels/PC at `v`; `None` ⇒ VMA == LMA.
    fn switch_section(&mut self, name: &str, cpu: Cpu, vma_base: Option<u32>);
    /// Emit a run of bytes with pending fixups at the current position.
    fn emit_data(&mut self, bytes: &[u8], fixups: Vec<Fixup>, span: Span);
    /// Emit `count` copies of `value` (gap fill / padding).
    fn emit_fill(&mut self, count: u32, value: u8, span: Span);
    /// Reserve `count` bytes of address space with NO image bytes (`ds` under phase).
    fn reserve(&mut self, count: u32, span: Span);
    /// Push a raw `Fragment` built elsewhere (not from raw bytes via
    /// `emit_data`) at the current position, advancing the section cursor by
    /// `advance` bytes. This is the M1.C T5c door for `Fragment::JmpJsrSym`:
    /// its real length is chosen later by the linker's `resolve_layout`, so
    /// `advance` must be the abs.w BASELINE width (4) that `resolve_layout`
    /// assumes when it shifts subsequent label offsets (see
    /// `sigil-link/src/relax.rs::shift_breakpoints`) — passing anything else
    /// would desync this front-end's label offsets from that assumption.
    fn emit_fragment(&mut self, frag: Fragment, advance: u32);
    /// Record a label at the current position.
    fn define_label(&mut self, name: &str);
    /// Record a diagnostic.
    fn diag(&mut self, d: Diagnostic);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DataFragment, Fixup, FixupKind};
    use crate::expr::Expr;
    use sigil_span::{SourceId, Span};

    // A trivial backend proving the trait is object-shaped and CPU-agnostic.
    struct FakeBackend;
    impl Backend for FakeBackend {
        type Mnemonic = u8;
        type Operand = u8;
        fn cpu(&self) -> Cpu {
            Cpu::Z80
        }
        fn lower(&self, m: u8, ops: &[u8], span: Span) -> Result<DataFragment, LowerError> {
            // Pretend `m` is an opcode and each op is a trailing byte.
            let mut bytes = vec![m];
            bytes.extend_from_slice(ops);
            Ok(DataFragment { bytes, fixups: vec![], span })
        }
    }

    #[test]
    fn backend_lowers_to_fragment() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let b = FakeBackend;
        let frag = b.lower(0x3E, &[0x0C], span).unwrap();
        assert_eq!(b.cpu(), Cpu::Z80);
        assert_eq!(frag.bytes, vec![0x3E, 0x0C]);
    }

    #[test]
    fn lower_error_carries_message() {
        let e = LowerError { message: "unsupported form: ld (nn),ir".to_string() };
        assert!(e.message.contains("unsupported"));
    }

    // A trivial IrStreamer consumer proving the trait's shape.
    #[derive(Default)]
    struct Collector {
        bytes: Vec<u8>,
        fixups: Vec<Fixup>,
    }
    impl IrStreamer for Collector {
        fn switch_section(&mut self, _name: &str, _cpu: Cpu, _vma_base: Option<u32>) {}
        fn emit_data(&mut self, bytes: &[u8], fixups: Vec<Fixup>, _span: Span) {
            self.bytes.extend_from_slice(bytes);
            self.fixups.extend(fixups);
        }
        fn emit_fill(&mut self, _count: u32, _value: u8, _span: Span) {}
        fn reserve(&mut self, _count: u32, _span: Span) {}
        fn emit_fragment(&mut self, _frag: Fragment, _advance: u32) {}
        fn define_label(&mut self, _name: &str) {}
        fn diag(&mut self, _d: Diagnostic) {}
    }

    #[test]
    fn ir_streamer_collects_data_and_fixups() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let mut c = Collector::default();
        c.emit_data(
            &[0x11, 0x00, 0x00],
            vec![Fixup { kind: FixupKind::BankPtr16Le, offset: 1, target: Expr::Int(0x845F) }],
            span,
        );
        assert_eq!(c.bytes, vec![0x11, 0x00, 0x00]);
        assert_eq!(c.fixups.len(), 1);
    }
}
