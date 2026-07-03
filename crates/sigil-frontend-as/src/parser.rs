//! parser: lexed line → structural `Line`.
#![allow(dead_code)] // removed once eval consumes this

use crate::ast::Line;
use crate::token::{Punct, Tok, Token};

/// Structure a lexed line: peel an explicit `Name:` label; keep the rest as
/// tokens for eval.
pub fn parse_line_tokens(toks: &[Token]) -> Line {
    if let (Some(a), Some(b)) = (toks.first(), toks.get(1)) {
        if matches!(&a.tok, Tok::Ident(_)) && matches!(&b.tok, Tok::Punct(Punct::Colon)) {
            let name = match &a.tok {
                Tok::Ident(s) => s.clone(),
                _ => unreachable!(),
            };
            return Line { label_colon: Some(name), tokens: toks[2..].to_vec() };
        }
    }
    Line { label_colon: None, tokens: toks.to_vec() }
}

#[cfg(test)]
mod tests {
    use super::parse_line_tokens;
    use crate::lexer::lex_line;
    use crate::token::Tok;
    use sigil_ir::backend::Cpu;
    use sigil_span::SourceId;

    fn line(src: &str) -> (Option<String>, Vec<Tok>) {
        let toks = lex_line(src, Cpu::Z80, SourceId(0), 0).unwrap();
        let l = parse_line_tokens(&toks);
        (l.label_colon, l.tokens.iter().map(|t| t.tok.clone()).collect())
    }

    #[test]
    fn explicit_colon_label_split() {
        let (lbl, toks) = line("Start: nop");
        assert_eq!(lbl, Some("Start".into()));
        assert_eq!(toks, vec![Tok::Ident("nop".into())]);
    }

    #[test]
    fn no_colon_label_stays_in_tokens() {
        // eval decides whether the first bareword is a label or an op.
        let (lbl, toks) = line("SeqTable db 0");
        assert_eq!(lbl, None);
        assert_eq!(toks, vec![Tok::Ident("SeqTable".into()), Tok::Ident("db".into()), Tok::Int(0)]);
    }

    #[test]
    fn bare_instruction() {
        let (lbl, toks) = line("ld a,(hl)");
        assert_eq!(lbl, None);
        assert_eq!(toks.first(), Some(&Tok::Ident("ld".into())));
    }
}
