//! The modern .emp front-end: lexer + parser + AST (Spec 2, Plan 1) + the
//! evaluator (Plans 2-3) and IR lowering (Plan 4). Only [`lower`] imports the
//! Core IR / backend crates (D-P4.1); the evaluator (`value`, `eval`, `layout`)
//! stays Core-free.
pub mod ast;
pub mod branch_const;
pub mod calls;
pub mod closure;
pub mod context;
pub mod contract;
pub mod corpus_contracts;
pub mod cycle_budget;
pub mod export_side;
pub mod eval;
pub mod flag_check;
pub mod layout;
pub mod lexer;
pub mod lower;
pub mod m68k_cycles;
pub mod mul_lower;
pub mod out_verify;
pub mod parser;
pub mod preserves;
pub mod regfile;
pub mod resolve;
pub mod resumable;
#[cfg_attr(not(test), allow(dead_code))] // Task 3 of step A piece 2a walks routines with it; remove then.
mod sr_level;
pub mod z80_preserves;
pub mod type_slice;
pub mod value;
pub mod z80_bus;
pub mod z80_closure;
pub mod z80_cycles;
pub mod z80_out_verify;

use sigil_span::{Diagnostic, SourceId};

/// Where a comptime file read (`embed`/`import`) resolves: one directory every path
/// joins (the single-root form, for `sigil emp --root`, a seam's own sound directory and
/// single-module port tests), or a build's named roots (G0b Task 9,
/// [`sigil_span::roots`]). The build's `generated` root is one of the named roots, so a
/// build's generated files are reached only through a lowering's own options, never
/// through state outside them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sandbox {
    /// One directory: a relative path joins it (or the lowering's `embed_base`).
    Dir(std::path::PathBuf),
    /// Named roots: a path names its root by its first segment.
    Roots(std::sync::Arc<sigil_span::roots::SourceRoots>),
}

impl From<std::path::PathBuf> for Sandbox {
    fn from(dir: std::path::PathBuf) -> Sandbox {
        Sandbox::Dir(dir)
    }
}

impl From<&std::path::Path> for Sandbox {
    fn from(dir: &std::path::Path) -> Sandbox {
        Sandbox::Dir(dir.to_path_buf())
    }
}

/// Convenience entry: lex + parse one source string as `SourceId(0)`.
pub fn parse_str(src: &str) -> (ast::File, Vec<Diagnostic>) {
    parse_file(src, SourceId(0))
}

/// Lex + parse `src`, attributed to `source`, returning the parsed [`ast::File`]
/// and every diagnostic collected from lexing and parsing.
pub fn parse_file(src: &str, source: SourceId) -> (ast::File, Vec<Diagnostic>) {
    let (tokens, lex_errs) = lexer::lex(src, source);
    let mut p = parser::Parser::new(tokens, src);
    for e in lex_errs {
        p.diag_at(e.span, e.message);
    }
    let file = p.file();
    (file, p.into_diagnostics())
}
