//! token: the lexer's output vocabulary.

// Consumed by the parser (next task); unused until then.

use sigil_span::Span;

/// A punctuation / operator token.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Punct {
    Plus, Minus, Star, Slash,
    Shl, Shr, Amp, Pipe,
    Eq, Ne, Lt, Gt, Le, Ge,
    OrOr, AndAnd,
    LParen, RParen, Comma, Colon,
    /// `#` — 68k immediate marker (`#expr`).
    Hash,
    /// `:=` — AS reassignable-symbol assignment (`name := expr`, same as `set`).
    /// Must be lexed as ONE token (maximal munch) so a leading `:` is never
    /// mistaken for a colon-label by `parse_line_tokens`.
    ColonEq,
}

/// A lexical token kind.
///
/// No `Eq`/`Hash` (only `PartialEq`): `Float` carries an `f64`, which has
/// no total order. Nothing keys a map on `Tok`/`Token` today.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// Identifier (mnemonic, label, symbol, directive keyword). Case-sensitive.
    Ident(String),
    /// Resolved integer literal (hex or decimal).
    Int(i64),
    /// A decimal float literal (`6.283185307179586`) — only meaningful inside
    /// a `sin(...)`/`int(...)` builtin call argument (§7.4: these are
    /// FRONT-END-only builtins, never `sigil_ir::Expr`; see
    /// `eval.rs::eval_float`). Elsewhere it is a parse error, same as any
    /// other unrecognized atom.
    Float(f64),
    /// String literal contents (raw, quotes stripped; escapes NOT processed).
    Str(String),
    /// `$` location counter (Z80 context only).
    Dollar,
    /// A punctuation / operator.
    Punct(Punct),
}

/// A token plus its source span.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}
