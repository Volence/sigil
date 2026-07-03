//! ast: structural line produced by the parser and consumed by eval.
#![allow(dead_code)] // removed once eval consumes Line

use crate::token::Token;

/// One source line, minimally structured. `label_colon` is an explicit `Name:`
/// definition; the remaining `tokens` (op + operands, or a bare-label form) are
/// interpreted by eval, which owns the mnemonic/directive/macro tables.
#[derive(Clone, Debug)]
pub struct Line {
    pub label_colon: Option<String>,
    pub tokens: Vec<Token>,
}
