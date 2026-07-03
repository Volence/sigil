//! CPU tag and (Task 4) the Backend / IrStreamer traits.

/// Which instruction set a [`crate::Section`]'s bytes are encoded for.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Cpu {
    Z80,
    M68000,
}

use crate::{DataFragment, Fixup};
use sigil_span::Span;

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
/// single [`DataFragment`] directly; `jr`/`djnz` residual displacement fixups
/// are attached to that fragment and resolved by the linker.
pub trait Backend {
    type Mnemonic;
    type Operand;

    /// The CPU this backend encodes for.
    fn cpu(&self) -> Cpu;

    /// Lower one fully-decoded instruction to a data fragment (+ any residual
    /// fixups, e.g. `Z80JrRel8` for `jr`/`djnz`).
    fn lower(
        &self,
        mnemonic: Self::Mnemonic,
        operands: &[Self::Operand],
        span: Span,
    ) -> Result<DataFragment, LowerError>;
}

/// The streaming sink a front-end emits into (M0 subset of Core §4.9). Plan 4's
/// AS front-end will drive this; Plan 3 only defines it and proves its shape.
pub trait IrStreamer {
    /// Emit a run of bytes with pending fixups at the current position.
    fn emit_data(&mut self, bytes: &[u8], fixups: Vec<Fixup>, span: Span);

    /// Record a label at the current position.
    fn define_label(&mut self, name: &str);
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
        fn emit_data(&mut self, bytes: &[u8], fixups: Vec<Fixup>, _span: Span) {
            self.bytes.extend_from_slice(bytes);
            self.fixups.extend(fixups);
        }
        fn define_label(&mut self, _name: &str) {}
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
