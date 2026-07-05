//! Token-level substitution helpers for macro/function expansion, split out of
//! `eval.rs` to keep that module focused on the pass/dispatch core.

use crate::token::{Punct, Tok, Token};

/// Reconstruct source text from a token slice. A space is inserted between two
/// tokens ONLY when omitting it would MERGE them on re-lex (both the left token's
/// last char and the right token's first char are identifier chars) — e.g. `move`
/// `d0` → `move d0`, but `#` `1` → `#1` (asl keeps the raw `#1`, no space). This
/// matters byte-for-byte when a rendered macro argument is substituted into a
/// STRING literal (debugger.asm's `%<…>` assert strings embed the `dest`/`src`
/// params verbatim): a spurious space would become a literal byte. Used for
/// `ALLARGS` / positional-arg substitution text.
pub(crate) fn render_tokens(toks: &[Token]) -> String {
    let mut out = String::new();
    for t in toks {
        let s = match &t.tok {
            Tok::Ident(x) => x.clone(),
            Tok::Int(n) => n.to_string(),
            Tok::Float(f) => f.to_string(),
            Tok::Str(x) => format!("\"{x}\""),
            Tok::Dollar => "$".to_string(),
            Tok::Punct(p) => punct_str(*p).to_string(),
        };
        if let (Some(prev), Some(next)) = (out.chars().last(), s.chars().next()) {
            if is_ident_char(prev) && is_ident_char(next) {
                out.push(' ');
            }
        }
        out.push_str(&s);
    }
    out
}

/// A character that can be part of an AS identifier/number — the boundary test
/// for whether two adjacent rendered tokens would merge on re-lex.
fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn punct_str(p: Punct) -> &'static str {
    match p {
        Punct::Plus => "+",
        Punct::Minus => "-",
        Punct::Star => "*",
        Punct::Slash => "/",
        Punct::Shl => "<<",
        Punct::Shr => ">>",
        Punct::Amp => "&",
        Punct::Pipe => "|",
        Punct::Eq => "=",
        Punct::Ne => "<>",
        Punct::Lt => "<",
        Punct::Gt => ">",
        Punct::Le => "<=",
        Punct::Ge => ">=",
        Punct::LParen => "(",
        Punct::RParen => ")",
        Punct::OrOr => "||",
        Punct::AndAnd => "&&",
        Punct::Comma => ",",
        Punct::Colon => ":",
        Punct::Hash => "#",
        Punct::ColonEq => ":=",
        Punct::Bang => "!",
        Punct::Tilde => "~",
    }
}

/// Whole-word text replace (identifier boundaries), for positional macro params.
pub(crate) fn replace_word(text: &str, word: &str, repl: &str) -> String {
    if word.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    let mut rest = text;
    while let Some(pos) = rest.find(word) {
        let before = &rest[..pos];
        let after = &rest[pos + word.len()..];
        let ok_before = before
            .chars()
            .last()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
        let ok_after = after
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
        out.push_str(before);
        if ok_before && ok_after {
            out.push_str(repl);
        } else {
            out.push_str(word);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Given `toks` with a `(` at index `lparen`, split the argument groups by
/// depth-0 commas and return `(args, index_past_matching_rparen)`. None if unbalanced.
pub(crate) fn split_call_args(toks: &[Token], lparen: usize) -> Option<(Vec<Vec<Token>>, usize)> {
    let mut depth = 0i32;
    let mut i = lparen;
    let mut args: Vec<Vec<Token>> = Vec::new();
    let mut cur: Vec<Token> = Vec::new();
    while i < toks.len() {
        match &toks[i].tok {
            Tok::Punct(Punct::LParen) => {
                depth += 1;
                if depth > 1 {
                    cur.push(toks[i].clone());
                }
                i += 1;
            }
            Tok::Punct(Punct::RParen) => {
                depth -= 1;
                if depth == 0 {
                    args.push(cur);
                    return Some((args, i + 1));
                }
                cur.push(toks[i].clone());
                i += 1;
            }
            Tok::Punct(Punct::Comma) if depth == 1 => {
                args.push(std::mem::take(&mut cur));
                i += 1;
            }
            _ => {
                cur.push(toks[i].clone());
                i += 1;
            }
        }
    }
    None
}

/// Split a token slice on top-level (non-parenthesised) commas.
pub(crate) fn split_top_commas(toks: &[Token]) -> Vec<&[Token]> {
    let mut groups = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, t) in toks.iter().enumerate() {
        match t.tok {
            Tok::Punct(Punct::LParen) => depth += 1,
            Tok::Punct(Punct::RParen) => depth -= 1,
            Tok::Punct(Punct::Comma) if depth == 0 => {
                groups.push(&toks[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    groups.push(&toks[start..]);
    groups
}
