//! The modern .emp front-end: lexer + parser + AST (Spec 2, Plan 1).
//! Lowering to IR is Plan 4; this crate depends on sigil-span ONLY.
pub mod ast;
pub mod lexer;
pub mod parser;
