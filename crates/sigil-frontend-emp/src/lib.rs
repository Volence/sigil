//! The modern .emp front-end: lexer + parser + AST (Spec 2, Plan 1).
//! Lowering to IR is Plan 4; this crate depends on sigil-span ONLY.
pub mod ast;
pub mod eval;
pub mod layout;
pub mod lexer;
pub mod parser;
pub mod value;

use sigil_span::{Diagnostic, SourceId};

/// Convenience entry: lex + parse one source string as `SourceId(0)`.
pub fn parse_str(src: &str) -> (ast::File, Vec<Diagnostic>) {
    parse_file(src, SourceId(0))
}

/// Lex + parse `src`, attributed to `source`, returning the parsed [`ast::File`]
/// and every diagnostic collected from lexing and parsing.
pub fn parse_file(src: &str, source: SourceId) -> (ast::File, Vec<Diagnostic>) {
    let (tokens, lex_errs) = lexer::lex(src, source);
    let mut p = parser::Parser::new(tokens);
    for e in lex_errs {
        p.diag_at(e.span, e.message);
    }
    let file = p.file();
    (file, p.into_diagnostics())
}
