//! expr: token-slice → `sigil_ir::Expr` with AS-flavoured precedence.

use crate::token::{Punct, Tok, Token};
use sigil_ir::expr::{BinOp, UnOp};
use sigil_ir::Expr;

/// Parse a leading expression from `toks`; return it plus the unconsumed tail.
/// `None` if the head is not an expression.
pub fn parse_expr(toks: &[Token]) -> Option<(Expr, &[Token])> {
    parse_bp(toks, 0)
}

/// Binding-power ladder: higher binds tighter.
fn infix_bp(p: Punct) -> Option<(u8, BinOp)> {
    use Punct::*;
    Some(match p {
        Star => (6, BinOp::Mul), Slash => (6, BinOp::Div),
        Plus => (5, BinOp::Add), Minus => (5, BinOp::Sub),
        Shl => (4, BinOp::Shl), Shr => (4, BinOp::Shr),
        Amp => (3, BinOp::And),
        Pipe => (2, BinOp::Or),
        Eq => (1, BinOp::Eq), Ne => (1, BinOp::Ne),
        Lt => (1, BinOp::Lt), Gt => (1, BinOp::Gt),
        Le => (1, BinOp::Le), Ge => (1, BinOp::Ge),
        _ => return None,
    })
}

fn parse_bp(toks: &[Token], min_bp: u8) -> Option<(Expr, &[Token])> {
    let (mut lhs, mut rest) = parse_atom(toks)?;
    while let Some(Tok::Punct(p)) = rest.first().map(|t| &t.tok) {
        let (bp, op) = match infix_bp(*p) {
            Some(x) if x.0 > min_bp => x,
            _ => break,
        };
        let (rhs, r2) = parse_bp(&rest[1..], bp)?;
        lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        rest = r2;
    }
    Some((lhs, rest))
}

fn parse_atom(toks: &[Token]) -> Option<(Expr, &[Token])> {
    let (head, rest) = toks.split_first()?;
    match &head.tok {
        Tok::Int(n) => Some((Expr::Int(*n), rest)),
        Tok::Dollar => Some((Expr::Sym("$".to_string()), rest)),
        Tok::Ident(name) => Some((Expr::Sym(name.clone()), rest)),
        Tok::Punct(Punct::Minus) => {
            let (inner, r) = parse_atom(rest)?;
            Some((Expr::Unary { op: UnOp::Neg, operand: Box::new(inner) }, r))
        }
        Tok::Punct(Punct::LParen) => {
            let (inner, r) = parse_bp(rest, 0)?;
            match r.first().map(|t| &t.tok) {
                Some(Tok::Punct(Punct::RParen)) => Some((inner, &r[1..])),
                _ => None, // unbalanced paren
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_expr;
    use crate::lexer::lex_line;
    use sigil_ir::backend::Cpu;
    use sigil_ir::expr::Fold;
    use sigil_span::SourceId;

    fn fold(src: &str, lookup: &dyn Fn(&str) -> Option<i64>) -> i64 {
        let toks = lex_line(src, Cpu::Z80, SourceId(0), 0).unwrap();
        let (e, rest) = parse_expr(&toks).unwrap();
        assert!(rest.is_empty(), "unconsumed tokens: {rest:?}");
        match e.fold(lookup) {
            Fold::Value(v) => v,
            Fold::Poison => panic!("poison"),
        }
    }

    #[test]
    fn arithmetic_and_precedence() {
        let none = |_: &str| None;
        assert_eq!(fold("2 + 3 * 4", &none), 14);
        assert_eq!(fold("(2 + 3) * 4", &none), 20);
        assert_eq!(fold("38h - 8", &none), 0x30);
        assert_eq!(fold("0FFh & 0F0h", &none), 0xF0);
        assert_eq!(fold("(0D69Ah & 7FFFh) | 8000h", &none), 0xD69A);
        assert_eq!(fold("1024 - (1000000000 / (59 * 18773))", &none), 122);
        assert_eq!(fold("-5 + 8", &none), 3);
    }

    #[test]
    fn symbols_and_dollar() {
        let env = |n: &str| match n { "Ids_End" => Some(0x8290), "Ids" => Some(0x8284), "$" => Some(0x38), _ => None };
        assert_eq!(fold("Ids_End - Ids", &env), 0x0C);
        assert_eq!(fold("38h - $", &env), 0); // $ bound to 0x38
    }
}
