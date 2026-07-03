//! sigil-frontend-as: the quarantined AS-syntax front-end (the byte-exact oracle).
//!
//! Reads Aeon's AS source and lowers it through `sigil_ir::IrStreamer` + the
//! public `sigil_backend_z80::Z80Backend` seam. It never touches the raw ISA
//! codec (`sigil-isa`) nor the linker (`sigil-link`).

mod ast;
mod eval;
mod expr;
mod lexer;
mod operands;
mod parser;
mod state;
mod token;

use sigil_ir::backend::Cpu;
use sigil_ir::Module;
use sigil_span::Diagnostic;

/// Assembly options: the seeded symbol environment + the CPU active before any
/// `cpu` directive.
#[derive(Clone, Debug)]
pub struct Options {
    /// CPU active before the first `cpu` directive. M0 snippets set `cpu z80`
    /// explicitly; default `Z80` for the Z80-only M0 build.
    pub initial_cpu: Cpu,
    /// Pre-seeded integer symbols: the reference `-D` defines and (later) the
    /// stubbed 68k leaf values. Names are case-sensitive.
    pub defines: Vec<(String, i64)>,
}

impl Default for Options {
    fn default() -> Self {
        Options { initial_cpu: Cpu::Z80, defines: Vec::new() }
    }
}

/// Assemble a single source string into an unlinked [`Module`] (sections carry
/// labels + symbolic fixups; the linker resolves addresses). Returns every
/// diagnostic on failure.
pub fn assemble(src: &str, opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    eval::run(src, opts)
}
