use sigil_span::{SourceId, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    Newline,
    LBrace, RBrace, LParen, RParen, LBracket, RBracket,
    Comma, Colon, Semi, Dot, At, Hash, Star, Plus, Minus, Slash, Percent,
    Amp, Pipe, Caret, Bang, Lt, Gt, Eq, Tilde,
    EqEq, Ne, Le, Ge, Shl, Shr, Arrow, DotDot, PlusPlus, AndAnd, OrOr,
    Eof,
}

impl Tok {
    /// Test helper — `Tok::ident("proc")`.
    pub fn ident(s: &str) -> Tok { Tok::Ident(s.to_string()) }
}

#[derive(Debug, Clone)]
pub struct Token { pub tok: Tok, pub span: Span }

#[derive(Debug)]
pub struct LexError { pub message: String, pub span: Span }

pub fn lex(src: &str, source: SourceId) -> (Vec<Token>, Vec<LexError>) {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut errs = Vec::new();
    let mut i = 0usize;
    let span = |s: usize, e: usize| Span { source, start: s as u32, end: e as u32 };
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
                while i < b.len() && b[i] != b'\n' { i += 1; }
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
                push!(Tok::Ident(src[s..i].to_string()), s, i);
            }
            b'0'..=b'9' | b'$' => { i = lex_number(src, b, i, source, &mut out, &mut errs); }
            b'"' => { i = lex_string(src, b, i, source, &mut out, &mut errs); }
            _ => {
                let two = if i + 1 < b.len() { &src[i..i + 2] } else { "" };
                let (tok, len) = match two {
                    "==" => (Tok::EqEq, 2), "!=" => (Tok::Ne, 2), "<=" => (Tok::Le, 2),
                    ">=" => (Tok::Ge, 2), "<<" => (Tok::Shl, 2), ">>" => (Tok::Shr, 2),
                    "->" => (Tok::Arrow, 2), ".." => (Tok::DotDot, 2), "++" => (Tok::PlusPlus, 2),
                    "&&" => (Tok::AndAnd, 2), "||" => (Tok::OrOr, 2),
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
                            errs.push(LexError { message: format!("unexpected character {:?}", c as char), span: span(s, s + 1) });
                            i += 1;
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
    let span = |s: usize, e: usize| Span { source, start: s as u32, end: e as u32 };
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
        i += 2;
        let ds = i;
        while i < b.len() && (b[i] == b'0' || b[i] == b'1') { i += 1; }
        if ds == i {
            errs.push(LexError { message: "expected binary digits after `0b`".into(), span: span(s, i) });
            return i;
        }
        match i64::from_str_radix(&src[ds..i], 2) {
            Ok(v) => out.push(Token { tok: Tok::Int(v), span: span(s, i) }),
            Err(_) => errs.push(LexError { message: "binary literal out of range".into(), span: span(s, i) }),
        }
        return i;
    }
    while i < b.len() && b[i].is_ascii_digit() { i += 1; }
    // float: dot followed by a digit (so `0..256` stays Int DotDot Int)
    if i + 1 < b.len() && b[i] == b'.' && b[i + 1].is_ascii_digit() {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() { i += 1; }
        match src[s..i].parse::<f64>() {
            Ok(v) => out.push(Token { tok: Tok::Float(v), span: span(s, i) }),
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

fn lex_string(src: &str, b: &[u8], mut i: usize, source: SourceId,
              out: &mut Vec<Token>, errs: &mut Vec<LexError>) -> usize {
    let s = i;
    let span = |s: usize, e: usize| Span { source, start: s as u32, end: e as u32 };
    i += 1; // opening quote
    let mut val = String::new();
    while i < b.len() && b[i] != b'"' && b[i] != b'\n' {
        if b[i] == b'\\' && i + 1 < b.len() {
            match b[i + 1] {
                b'n' => val.push('\n'),
                b't' => val.push('\t'),
                b'\\' => val.push('\\'),
                b'"' => val.push('"'),
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
