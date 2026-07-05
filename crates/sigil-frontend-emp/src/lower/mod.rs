//! Lowering the checked, CPU-neutral `.emp` evaluator output into the Core IR
//! (Spec 2, Plan 4). This is the ONLY module in the crate that may import
//! `sigil_ir` / the backend crates (design decision D-P4.1): the pure evaluator
//! (`value`, `eval`, `layout`) stays Core-free so it can be tested in isolation.
//!
//! T0 proves the seam with the thinnest working path: `data` items only, one
//! 68000 section, defer-to-link placement (D-P4.2) — every pointer field becomes
//! a symbolic `Abs32Be` fixup the linker resolves. Instruction lowering and the
//! real per-CPU byte-order / fixup-kind table are T2+.

use crate::ast;
use crate::layout::eval_data;
use crate::value::{Cell, DataBuf};
use sigil_ir::backend::{Cpu, IrStreamer};
use sigil_ir::{Expr, Fixup, FixupKind, IrBuilder, Module};
use sigil_span::Diagnostic;

/// Options controlling how a `.emp` module lowers to Core IR.
pub struct LowerOptions {
    /// The CPU the initial section is encoded for.
    pub initial_cpu: Cpu,
}

/// Lower every `data` item in `file` into a single Core IR section, returning the
/// finished [`Module`] plus any diagnostics (from evaluation or lowering).
///
/// T0 scope: `data` items only. Each item's checked [`DataBuf`] is emitted as one
/// `emit_data` call under a label named after the item, so `sigil-link` can
/// resolve references to it.
pub fn lower_module(file: &ast::File, opts: &LowerOptions) -> (Module, Vec<Diagnostic>) {
    let mut builder = IrBuilder::new();
    let mut diags = Vec::new();

    builder.switch_section("text", opts.initial_cpu, None);

    for item in &file.items {
        let ast::Item::Data(decl) = item else { continue };
        let (buf, mut ds) = eval_data(file, &decl.name);
        diags.append(&mut ds);
        let Some(buf) = buf else { continue };

        let (bytes, fixups) = data_to_bytes(&buf);
        builder.define_label(&decl.name);
        builder.emit_data(&bytes, fixups, decl.span);
    }

    let (module, mut build_diags) = builder.finish();
    diags.append(&mut build_diags);
    (module, diags)
}

/// Serialize a checked [`DataBuf`] to image bytes plus fixups (offsets relative
/// to the start of this buffer, i.e. within the `DataFragment` it becomes).
///
/// **T0 stub.** Scalars are emitted big-endian, which is correct for M68000 (the
/// only CPU T0 lowers). T2 generalizes byte order to be CPU-driven and grows the
/// fixup-kind selection into a real table; for now every pointer is `Abs32Be`
/// (the Abs32 default — D-P3.7).
fn data_to_bytes(buf: &DataBuf) -> (Vec<u8>, Vec<Fixup>) {
    let mut bytes = Vec::with_capacity(buf.size);
    let mut fixups = Vec::new();

    for cell in &buf.cells {
        match cell {
            Cell::Scalar { value, width, .. } => {
                // Big-endian (M68000 order): the low `width` bytes, MSB first.
                let w = *width as usize;
                bytes.extend_from_slice(&value.to_be_bytes()[16 - w..]);
            }
            Cell::Bytes(b) => bytes.extend_from_slice(b),
            Cell::SymRef { name, width } => {
                // The reserved hole is sized from the fixup kind so the two never
                // drift; `width` is the DataBuf's independent record of the same
                // fact (both 4 for Abs32 today — D-P3.7). When T2 adds Abs16Be /
                // BankPtr16Le, kind selection must key off `width`.
                let kind = FixupKind::Abs32Be;
                debug_assert_eq!(*width as u32, kind.byte_width());
                fixups.push(Fixup { kind, offset: bytes.len() as u32, target: Expr::Sym(name.clone()) });
                bytes.resize(bytes.len() + kind.byte_width() as usize, 0);
            }
        }
    }

    (bytes, fixups)
}
