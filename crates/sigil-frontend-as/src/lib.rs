//! sigil-frontend-as: the quarantined AS-syntax front-end (the byte-exact oracle).
//!
//! Reads Aeon's AS source and lowers it through `sigil_ir::IrStreamer` + the
//! public `sigil_backend_z80::Z80Backend` seam. It never touches the raw ISA
//! codec (`sigil-isa`) nor the linker (`sigil-link`).

mod ast;
mod eval;
mod expand;
mod expr;
mod lexer;
mod operands;
mod parser;
mod state;
mod token;

use std::path::Path;

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
    /// stubbed 68k leaf values. Names are case-sensitive. These keep asl's
    /// silent-override semantics — an in-file `=`/`equ` of the same name wins
    /// (the code-gate / game-config-override defines rely on this).
    pub defines: Vec<(String, i64)>,
    /// Pre-seeded integer symbols the residual AS may NOT redefine — the
    /// `.emp`-owned constants injected by the Stage-3 P5 ownership flip (the
    /// harvest of `engine.constants`'s `pub const`s). Like [`defines`] they seed
    /// the env, but an in-file `=`/`equ` of a guarded name is a hard
    /// `[defines.collision]` error, not a silent override — the structural
    /// no-silent-shadowing guard proving a flipped constant has exactly ONE
    /// author (the `.emp` module). Distinct from [`defines`] precisely because
    /// the code-gate/override defines DO coexist with in-file definitions.
    pub guarded_defines: Vec<(String, i64)>,
    /// Directory that `include` paths resolve against. Set automatically by
    /// [`assemble_root`] from the root file's parent when left `None`.
    pub include_root: Option<std::path::PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            initial_cpu: Cpu::Z80,
            defines: Vec::new(),
            guarded_defines: Vec::new(),
            include_root: None,
        }
    }
}

/// Assemble a single source string into an unlinked [`Module`] (sections carry
/// labels + symbolic fixups; the linker resolves addresses). Returns every
/// diagnostic on failure.
pub fn assemble(src: &str, opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    eval::run(src, opts)
}

/// Assemble a root source file, resolving `include` paths relative to its parent
/// directory (unless `opts.include_root` is already set).
pub fn assemble_root(root: &Path, opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    assemble_root_impl(root, opts, false)
}

/// Like [`assemble_root`] but keeps section-label references SYMBOLIC through the final
/// pass so a later relocation (the harness's chained placement)
/// resolves them against each label's placed base. Use for a build whose sections will
/// MOVE after assembly; a pinned build must use [`assemble_root`] (byte-for-byte asl).
pub fn assemble_root_relocating(root: &Path, opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    assemble_root_impl(root, opts, true)
}

fn assemble_root_impl(root: &Path, opts: &Options, relocate: bool) -> Result<Module, Vec<Diagnostic>> {
    let text = std::fs::read_to_string(root).map_err(|e| {
        vec![sigil_span::Diagnostic {
            level: sigil_span::Level::Error,
            message: format!("cannot read {}: {e}", root.display()),
            primary: sigil_span::Span {
                source: sigil_span::SourceId(0),
                start: 0,
                end: 0,
            },
        }]
    })?;
    let mut o = opts.clone();
    if o.include_root.is_none() {
        o.include_root = root.parent().map(|p| p.to_path_buf());
    }
    if relocate {
        eval::run_relocating(&text, &o)
    } else {
        eval::run(&text, &o)
    }
}
