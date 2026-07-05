//! Lowering the checked, CPU-neutral `.emp` evaluator output into the Core IR
//! (Spec 2, Plan 4). This is the ONLY module in the crate that may import
//! `sigil_ir` / the backend crates (design decision D-P4.1): the pure evaluator
//! (`value`, `eval`, `layout`) stays Core-free so it can be tested in isolation.
//!
//! T0 proves the seam with the thinnest working path: `data` items only, one
//! section, defer-to-link placement (D-P4.2) — every pointer field becomes a
//! symbolic fixup the linker resolves. T2 grows the real per-CPU byte-order /
//! fixup-kind serializer, which lives in [`data`]. Instruction lowering is T3+.

mod code;
mod data;
mod proc;

pub use code::lower_code_buf;

use crate::ast;
use crate::layout::eval_data;
use sigil_ir::backend::{Cpu, IrStreamer};
use sigil_ir::{IrBuilder, Module};
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

    // Walk items in declaration order. Data emits its serialized buffer; a proc
    // emits its label + lowered body (T4). The proc arm needs the item's index
    // to check declared-fallthrough adjacency against the following item.
    for (index, item) in file.items.iter().enumerate() {
        match item {
            ast::Item::Data(decl) => {
                let (buf, mut ds) = eval_data(file, &decl.name);
                diags.append(&mut ds);
                let Some(buf) = buf else { continue };

                let (bytes, fixups, mut stream_diags) =
                    data::stream_data(&buf, opts.initial_cpu, decl.span);
                diags.append(&mut stream_diags);
                builder.define_label(&decl.name);
                builder.emit_data(&bytes, fixups, decl.span);
            }
            ast::Item::Proc(decl) => {
                proc::lower_proc(
                    file,
                    decl,
                    index,
                    &file.items,
                    opts.initial_cpu,
                    &mut builder,
                    &mut diags,
                );
            }
            _ => {}
        }
    }

    let (module, mut build_diags) = builder.finish();
    diags.append(&mut build_diags);
    (module, diags)
}
