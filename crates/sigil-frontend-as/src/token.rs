//! token: the lexer's output vocabulary.

// Consumed by the parser (next task); unused until then.

use sigil_span::Span;

/// A punctuation / operator token.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Punct {
    Plus, Minus, Star, Slash,
    Shl, Shr, Amp, Pipe,
    Eq, Ne, Lt, Gt, Le, Ge,
    LParen, RParen, Comma, Colon,
}

/// A lexical token kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Tok {
    /// Identifier (mnemonic, label, symbol, directive keyword). Case-sensitive.
    Ident(String),
    /// Resolved integer literal (hex or decimal).
    Int(i64),
    /// String literal contents (raw, quotes stripped; escapes NOT processed).
    Str(String),
    /// `$` location counter (Z80 context only).
    Dollar,
    /// A punctuation / operator.
    Punct(Punct),
}

/// A token plus its source span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}
