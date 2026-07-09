//! Lexer for the modern `.emp` language: source text in, spanned tokens out.

use sigil_span::{SourceId, Span};

/// A lexical token kind, carrying its payload for identifiers and literals.
///
/// Multi-character operators (`==`, `->`, `..`, ...) are single tokens.
/// `.` is always its own [`Tok::Dot`] token, so forms like `move.b` and
/// `.draw:` decompose here and are reassembled by the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// An identifier or keyword (keywords are not distinguished by the lexer).
    Ident(String),
    /// An integer literal: decimal, `$`-hex, or binary (`0b`-prefixed or
    /// `%`-prefixed, when `%` is immediately followed by a binary digit).
    Int(i64),
    /// A floating-point literal (digits, `.`, digits).
    Float(f64),
    /// A double-quoted string literal with escapes already resolved.
    Str(String),
    /// One or more consecutive line breaks, collapsed into a single token.
    Newline,
    /// A `///` doc-comment line (S2-D11(d)): the text after `///` with one
    /// optional leading space stripped. `//` and `////`+ stay ordinary
    /// (discarded) comments — exactly three slashes is the doc form, the
    /// Rust precedent.
    DocLine(String),
    LBrace, RBrace, LParen, RParen, LBracket, RBracket,
    Comma, Colon, Semi, Dot, At, Hash, Star, Plus, Minus, Slash, Percent,
    Amp, Pipe, Caret, Bang, Lt, Gt, Eq, Tilde,
    EqEq, Ne, Le, Ge, Shl, Shr, Arrow, DotDot, PlusPlus, AndAnd, OrOr,
    /// The pipe operator `|>` (function application, D-P2.17). Matched before
    /// single `|` so it is never mis-lexed as `Pipe` then `Gt`.
    PipeGt,
    /// The fat arrow `=>` (match arms, Spec 2 Plan 3). Matched before single
    /// `=` so it is never mis-lexed as `Eq` then `Gt`.
    FatArrow,
    /// End of input; always the final token emitted by [`lex`].
    Eof,
}

impl Tok {
    /// Test helper — `Tok::ident("proc")`.
    pub fn ident(s: &str) -> Tok { Tok::Ident(s.to_string()) }
}

/// A [`Tok`] paired with the byte-range [`Span`] it was lexed from.
#[derive(Debug, Clone)]
pub struct Token {
    /// The token kind (and payload, for identifiers and literals).
    pub tok: Tok,
    /// Where in the source this token came from.
    pub span: Span,
}

/// A lexical error: malformed literal, unknown escape, stray character, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    /// Human-readable description of the problem.
    pub message: String,
    /// The offending source range.
    pub span: Span,
}

/// Build a [`Span`] in `source` covering bytes `[s, e)`.
fn span_at(source: SourceId, s: usize, e: usize) -> Span {
    Span { source, start: s as u32, end: e as u32 }
}

/// Lex `src` into a token stream, collecting errors instead of failing fast.
///
/// Always returns a token vector ending in [`Tok::Eof`], even on errors —
/// erroneous input is skipped and lexing continues, so callers get the best
/// possible token stream plus every [`LexError`] encountered.
pub fn lex(src: &str, source: SourceId) -> (Vec<Token>, Vec<LexError>) {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut errs = Vec::new();
    let mut i = 0usize;
    let span = |s: usize, e: usize| span_at(source, s, e);
    macro_rules! push { ($t:expr, $s:expr, $e:expr) => { out.push(Token { tok: $t, span: span($s, $e) }) } }

    while i < b.len() {
        let s = i;
        let c = b[i];
        match c {
            b' ' | b'\t' | b'\r' => { i += 1; }
            b'\n' => {
                i += 1;
                // collapse runs of newlines into one token
                if !matches!(out.last(), Some(Token { tok: Tok::Newline, .. })) {
                    push!(Tok::Newline, s, i);
                }
            }
            b'/' if i + 1 < b.len() && b[i + 1] == b'/' => {
                // `///` (exactly three slashes) is a DOC line (S2-D11(d)),
                // tokenized so the parser can attach it to the next item;
                // `//` and `////`+ remain trivia, discarded here.
                let doc = i + 2 < b.len()
                    && b[i + 2] == b'/'
                    && !(i + 3 < b.len() && b[i + 3] == b'/');
                let text_start = i + 3;
                while i < b.len() && b[i] != b'\n' { i += 1; }
                if doc {
                    let mut t = &src[text_start.min(i)..i];
                    // CRLF: the scan stops at `\n` only, so strip the `\r`
                    // FIRST (then the one optional leading space) — doc text
                    // must never be the one place `\r` survives lexing.
                    t = t.strip_suffix('\r').unwrap_or(t);
                    t = t.strip_prefix(' ').unwrap_or(t);
                    push!(Tok::DocLine(t.to_string()), s, i);
                }
            }
            b'/' if i + 1 < b.len() && b[i + 1] == b'*' => {
                i += 2;
                let mut closed = false;
                while i + 1 < b.len() {
                    if b[i] == b'*' && b[i + 1] == b'/' { i += 2; closed = true; break; }
                    i += 1;
                }
                if !closed {
                    errs.push(LexError { message: "unterminated block comment".into(), span: span(s, b.len()) });
                    i = b.len();
                }
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') { i += 1; }
                // A single trailing apostrophe is part of the identifier: the Z80
                // shadow-register syntax `af'`/`bc'`/`de'`/`hl'`. Exactly one is
                // absorbed (Z80 uses exactly one); a second `'` is left to the
                // stray-character path so `af''` still errors.
                if i < b.len() && b[i] == b'\'' { i += 1; }
                push!(Tok::Ident(src[s..i].to_string()), s, i);
            }
            b'0'..=b'9' | b'$' => { i = lex_number(src, b, i, source, &mut out, &mut errs); }
            // `%` immediately followed by a binary digit is a binary literal
            // (`%1010`); `%` followed by anything else (whitespace, `2`..`9`,
            // a letter, EOF, ...) stays the modulo operator (`7 % 3`), handled
            // by the catch-all operator arm below.
            b'%' if i + 1 < b.len() && matches!(b[i + 1], b'0' | b'1') => {
                i = finish_binary(src, b, i, i + 1, source, &mut out, &mut errs);
            }
            b'"' => { i = lex_string(src, b, i, source, &mut out, &mut errs); }
            // A `'` reaching the top of the loop is never the Z80
            // shadow-register apostrophe — the identifier arm above already
            // absorbed that one trailing quote as part of the ident. So a
            // bare `'` here always starts a char literal (raw ASCII, Task 3).
            b'\'' => { i = lex_char(src, b, i, source, &mut out, &mut errs); }
            _ => {
                // `get` returns None off a char boundary — no panic on non-ASCII.
                let two = src.get(i..i + 2).unwrap_or("");
                let (tok, len) = match two {
                    "==" => (Tok::EqEq, 2), "!=" => (Tok::Ne, 2), "<=" => (Tok::Le, 2),
                    ">=" => (Tok::Ge, 2), "<<" => (Tok::Shl, 2), ">>" => (Tok::Shr, 2),
                    "->" => (Tok::Arrow, 2), ".." => (Tok::DotDot, 2), "++" => (Tok::PlusPlus, 2),
                    "&&" => (Tok::AndAnd, 2), "||" => (Tok::OrOr, 2),
                    // `|>` before the single-`|` fallback: distinct strings, so
                    // match order is irrelevant, but keep it beside `||`.
                    "|>" => (Tok::PipeGt, 2),
                    "=>" => (Tok::FatArrow, 2),
                    _ => match c {
                        b'{' => (Tok::LBrace, 1), b'}' => (Tok::RBrace, 1),
                        b'(' => (Tok::LParen, 1), b')' => (Tok::RParen, 1),
                        b'[' => (Tok::LBracket, 1), b']' => (Tok::RBracket, 1),
                        b',' => (Tok::Comma, 1), b':' => (Tok::Colon, 1), b';' => (Tok::Semi, 1),
                        b'.' => (Tok::Dot, 1), b'@' => (Tok::At, 1), b'#' => (Tok::Hash, 1),
                        b'*' => (Tok::Star, 1), b'+' => (Tok::Plus, 1), b'-' => (Tok::Minus, 1),
                        b'/' => (Tok::Slash, 1), b'%' => (Tok::Percent, 1), b'&' => (Tok::Amp, 1),
                        b'|' => (Tok::Pipe, 1), b'^' => (Tok::Caret, 1), b'!' => (Tok::Bang, 1),
                        b'<' => (Tok::Lt, 1), b'>' => (Tok::Gt, 1), b'=' => (Tok::Eq, 1),
                        b'~' => (Tok::Tilde, 1),
                        _ => {
                            // Advance by the full char so multi-byte UTF-8 never
                            // leaves `i` mid-character.
                            let ch = src[i..].chars().next().unwrap();
                            errs.push(LexError { message: format!("unexpected character {ch:?}"), span: span(s, s + ch.len_utf8()) });
                            i += ch.len_utf8();
                            continue;
                        }
                    },
                };
                i += len;
                push!(tok, s, i);
            }
        }
    }
    push!(Tok::Eof, b.len(), b.len());
    (out, errs)
}

fn lex_number(src: &str, b: &[u8], mut i: usize, source: SourceId,
              out: &mut Vec<Token>, errs: &mut Vec<LexError>) -> usize {
    let s = i;
    let span = |s: usize, e: usize| span_at(source, s, e);
    if b[i] == b'$' {
        i += 1;
        let ds = i;
        while i < b.len() && b[i].is_ascii_hexdigit() { i += 1; }
        if ds == i {
            errs.push(LexError { message: "expected hex digits after `$`".into(), span: span(s, i) });
            return i;
        }
        match i64::from_str_radix(&src[ds..i], 16) {
            Ok(v) => out.push(Token { tok: Tok::Int(v), span: span(s, i) }),
            Err(_) => errs.push(LexError { message: "hex literal out of range".into(), span: span(s, i) }),
        }
        return i;
    }
    if b[i] == b'0' && i + 1 < b.len() && b[i + 1] == b'b' {
        let ds = i + 2;
        if ds >= b.len() || !matches!(b[ds], b'0' | b'1') {
            errs.push(LexError { message: "expected binary digits after `0b`".into(), span: span(s, ds) });
            return ds;
        }
        return finish_binary(src, b, s, ds, source, out, errs);
    }
    while i < b.len() && b[i].is_ascii_digit() { i += 1; }
    // float: dot followed by a digit (so `0..256` stays Int DotDot Int)
    if i + 1 < b.len() && b[i] == b'.' && b[i + 1].is_ascii_digit() {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() { i += 1; }
        match src[s..i].parse::<f64>() {
            Ok(v) => out.push(Token { tok: Tok::Float(v), span: span(s, i) }),
            // Defensive: the slice is digits `.` digits, which f64 always
            // parses; unreachable in practice.
            Err(_) => errs.push(LexError { message: "bad float literal".into(), span: span(s, i) }),
        }
        return i;
    }
    match src[s..i].parse::<i64>() {
        Ok(v) => out.push(Token { tok: Tok::Int(v), span: span(s, i) }),
        Err(_) => errs.push(LexError { message: "integer literal out of range".into(), span: span(s, i) }),
    }
    i
}

/// Scan the maximal run of binary digits starting at `ds`, parse it as a
/// radix-2 integer, and push either an [`Tok::Int`] token or an out-of-range
/// error whose span runs from `s` (the literal's first byte, `%` or `0`).
///
/// The single home for the binary digit-run loop, the `from_str_radix(_, 2)`
/// parse, and the "binary literal out of range" message, shared by the `0b`
/// and `%` syntaxes so they can never drift. Callers guarantee `b[ds]` is a
/// binary digit, so the run is non-empty. Returns the index past the last
/// consumed digit.
fn finish_binary(src: &str, b: &[u8], s: usize, ds: usize, source: SourceId,
                 out: &mut Vec<Token>, errs: &mut Vec<LexError>) -> usize {
    let span = |s: usize, e: usize| span_at(source, s, e);
    let mut i = ds;
    while i < b.len() && (b[i] == b'0' || b[i] == b'1') { i += 1; }
    match i64::from_str_radix(&src[ds..i], 2) {
        Ok(v) => out.push(Token { tok: Tok::Int(v), span: span(s, i) }),
        Err(_) => errs.push(LexError { message: "binary literal out of range".into(), span: span(s, i) }),
    }
    i
}

fn lex_string(src: &str, b: &[u8], mut i: usize, source: SourceId,
              out: &mut Vec<Token>, errs: &mut Vec<LexError>) -> usize {
    let s = i;
    let span = |s: usize, e: usize| span_at(source, s, e);
    i += 1; // opening quote
    let mut val = String::new();
    while i < b.len() && b[i] != b'"' && b[i] != b'\n' {
        if b[i] == b'\\' && i + 1 < b.len() {
            match b[i + 1] {
                b'n' => val.push('\n'),
                b't' => val.push('\t'),
                b'\\' => val.push('\\'),
                b'"' => val.push('"'),
                // Author-controlled termination (lexical gaps, Task 4): a
                // string never gets an implicit trailing 0, but `\0` lets the
                // author write one explicitly (`"HELLO\0"`). Mirrors
                // `lex_char`'s escape set.
                b'0' => val.push('\0'),
                other => {
                    errs.push(LexError { message: format!("unknown escape \\{}", other as char), span: span(i, i + 2) });
                }
            }
            i += 2;
        } else {
            // multi-byte UTF-8 safe: copy the full char
            let ch = src[i..].chars().next().unwrap();
            val.push(ch);
            i += ch.len_utf8();
        }
    }
    if i >= b.len() || b[i] != b'"' {
        errs.push(LexError { message: "unterminated string".into(), span: span(s, i) });
        return i;
    }
    i += 1; // closing quote
    out.push(Token { tok: Tok::Str(val), span: span(s, i) });
    i
}

/// Lex a char literal `'A'` into a plain [`Tok::Int`] — raw ASCII, so `'A'` is
/// exactly the integer 65, flowing through the same paths as any other
/// integer literal (e.g. `data D = byte('A')` emits `$41`).
///
/// Charmap-based text encoding (for on-screen tile text) is a separate,
/// explicit opt-in future feature — out of scope here.
///
/// Mirrors [`lex_string`]'s scanning shape (escape handling, UTF-8-safe char
/// stepping, unterminated detection) but a char literal wants exactly one
/// character or escape between the quotes, and only ASCII (`0..=127`).
fn lex_char(src: &str, b: &[u8], mut i: usize, source: SourceId,
            out: &mut Vec<Token>, errs: &mut Vec<LexError>) -> usize {
    let s = i;
    let span = |s: usize, e: usize| span_at(source, s, e);
    i += 1; // opening quote

    let mut values: Vec<u8> = Vec::new();
    // A non-ASCII char or unknown escape is a more specific error than
    // "empty"/"multi-character" — once seen, keep scanning to resync at the
    // closing quote, but don't also report a count-based error afterward.
    let mut bad = false;
    while i < b.len() && b[i] != b'\'' && b[i] != b'\n' {
        if b[i] == b'\\' && i + 1 < b.len() {
            match b[i + 1] {
                b'n' => values.push(b'\n'),
                b't' => values.push(b'\t'),
                b'\\' => values.push(b'\\'),
                b'\'' => values.push(b'\''),
                b'0' => values.push(0),
                other => {
                    errs.push(LexError { message: format!("unknown escape \\{}", other as char), span: span(i, i + 2) });
                    bad = true;
                }
            }
            i += 2;
        } else {
            // multi-byte UTF-8 safe: step by the full char
            let ch = src[i..].chars().next().unwrap();
            if ch.is_ascii() {
                values.push(ch as u8);
            } else {
                errs.push(LexError {
                    message: format!(
                        "char literal must be ASCII; {ch:?} is not — use a numeric literal or an escape"
                    ),
                    span: span(i, i + ch.len_utf8()),
                });
                bad = true;
            }
            i += ch.len_utf8();
        }
    }

    if i >= b.len() || b[i] != b'\'' {
        errs.push(LexError { message: "unterminated char literal".into(), span: span(s, i) });
        return i;
    }
    i += 1; // closing quote

    if bad {
        // A more specific error was already reported above.
        return i;
    }
    match values.len() {
        0 => errs.push(LexError { message: "empty char literal".into(), span: span(s, i) }),
        1 => out.push(Token { tok: Tok::Int(values[0] as i64), span: span(s, i) }),
        n => errs.push(LexError {
            message: format!("char literal must be a single character; got {n}"),
            span: span(s, i),
        }),
    }
    i
}
