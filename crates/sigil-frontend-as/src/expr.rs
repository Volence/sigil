//! expr: token-slice → `sigil_ir::Expr` with AS-flavoured precedence.

use crate::token::{Punct, Tok, Token};
use sigil_ir::expr::{BinOp, UnOp};
use sigil_ir::Expr;

/// Maximum operand-nesting depth, mirroring the `.emp` front end's own limit.
///
/// Without a bound, `parse_atom`'s three recursive arms (`-x`, `~x`, `(x)`) are
/// limited only by input size, and a syntactically PERFECT `dc.b (((…1…)))` deep
/// enough overflows the native stack. That is a SIGABRT, not a panic: it cannot be
/// `catch_unwind`'d, so `sigil` dies with no diagnostic and no location. Measured
/// at 40,000 nesting levels before this guard (lens sweep, seat SAFE, finding
/// S19). Reachable from every operand and every `dc.b/w/l` in a frontend that
/// still assembles `game_root.asm` on every build.
///
/// The corpus nests single digits deep; 128 is far above any real expression and
/// far below the stack budget.
const MAX_EXPR_DEPTH: u32 = 128;

/// Parse a leading expression from `toks`; return it plus the unconsumed tail.
/// `None` if the head is not an expression, or if it nests past
/// [`MAX_EXPR_DEPTH`] — the same "not an expression here" answer the unbalanced-
/// paren arm already returns, so callers report a clean parse error either way.
pub fn parse_expr(toks: &[Token]) -> Option<(Expr, &[Token])> {
    parse_bp(toks, 0, 0)
}

/// Binding-power ladder: higher binds tighter.
///
/// `||` is loosest, `&&` binds tighter than `||` but looser than comparisons,
/// mirroring AS's real operator surface (empirically confirmed against `asl`:
/// both fold to a neutral `1`/`0`, same as the comparison tier).
fn infix_bp(p: Punct) -> Option<(u8, BinOp)> {
    use Punct::*;
    Some(match p {
        Star => (8, BinOp::Mul),
        Slash => (8, BinOp::Div),
        // `#` infix modulo — same precedence tier as `*`/`/` (asl-verified:
        // `7#5*2`=4, `5+7#2`=6). Distinct from the OPERAND-level `#expr`
        // immediate marker, which `operands.rs::classify` consumes from the
        // front of an operand group before this parser ever sees it — by the
        // time `parse_expr` runs, any remaining `#` is unambiguously infix.
        Hash => (8, BinOp::Mod),
        Plus => (7, BinOp::Add),
        Minus => (7, BinOp::Sub),
        Shl => (6, BinOp::Shl),
        Shr => (6, BinOp::Shr),
        Amp => (5, BinOp::And),
        Pipe => (4, BinOp::Or),
        // `!` — AS's infix bitwise XOR (asl-verified 2026-07-04: `1!1`=0,
        // `3!1`=2, `5!3`=6; the earlier bitwise-OR reading was wrong — the
        // only prior golden `3!4`=7 can't tell OR from XOR). Same tier as `|`.
        // Drives `__ErrorMessage`'s `.__align_flag: set (((*)&1)!1)*$80`.
        Bang => (4, BinOp::Xor),
        Eq => (3, BinOp::Eq),
        Ne => (3, BinOp::Ne),
        Lt => (3, BinOp::Lt),
        Gt => (3, BinOp::Gt),
        Le => (3, BinOp::Le),
        Ge => (3, BinOp::Ge),
        AndAnd => (2, BinOp::LogAnd),
        OrOr => (1, BinOp::LogOr),
        _ => return None,
    })
}

fn parse_bp(toks: &[Token], min_bp: u8, depth: u32) -> Option<(Expr, &[Token])> {
    let (mut lhs, mut rest) = parse_atom(toks, depth)?;
    while let Some(Tok::Punct(p)) = rest.first().map(|t| &t.tok) {
        let (bp, op) = match infix_bp(*p) {
            Some(x) if x.0 > min_bp => x,
            _ => break,
        };
        let (rhs, r2) = parse_bp(&rest[1..], bp, depth)?;
        lhs = Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
        rest = r2;
    }
    Some((lhs, rest))
}

fn parse_atom(toks: &[Token], depth: u32) -> Option<(Expr, &[Token])> {
    if depth >= MAX_EXPR_DEPTH {
        return None;
    }
    let depth = depth + 1;
    let (head, rest) = toks.split_first()?;
    match &head.tok {
        Tok::Int(n) => Some((Expr::Int(*n), rest)),
        Tok::Dollar => Some((Expr::Sym("$".to_string()), rest)),
        // A standalone `*` in atom (primary-expression) position is AS's other
        // spelling of the current-PC symbol (used by `pscStart := *` etc. in
        // `parallax_section`/`parallax_section_end`). `parse_atom` is only ever
        // invoked expecting a primary expression, so a `Star` reaching here is
        // unambiguous — it can't be the infix multiplication operator, which
        // `parse_bp`'s loop consumes only after a valid lhs. Folding it to the
        // same `Expr::Sym("$")` that `$` produces means every existing
        // `$`-handling site (front-end `fold`, poison detection) already
        // supports it with no further changes.
        Tok::Punct(Punct::Star) => Some((Expr::Sym("$".to_string()), rest)),
        Tok::Ident(name) => Some((Expr::Sym(name.clone()), rest)),
        Tok::Punct(Punct::Minus) => {
            let (inner, r) = parse_atom(rest, depth)?;
            Some((
                Expr::Unary {
                    op: UnOp::Neg,
                    operand: Box::new(inner),
                },
                r,
            ))
        }
        // `~expr` — prefix bitwise complement (asl-verified: `~$0F` = -16 =
        // `$FFFFFFF0`). Binds like negation (tighter than the binary tier), so
        // `~(mask)` / `~BLOCK_TILE_SIZE-1` parse as `(~x)` then any following
        // binary operator, matching asl.
        Tok::Punct(Punct::Tilde) => {
            let (inner, r) = parse_atom(rest, depth)?;
            Some((
                Expr::Unary {
                    op: UnOp::Not,
                    operand: Box::new(inner),
                },
                r,
            ))
        }
        // `~~expr` — prefix LOGICAL not, a distinct asl operator from `~`
        // (asl-verified 2026-09-03, both shipped builds agreeing:
        // `dc.b ~~0,~~1,~~5` = `01 00 00`, `dc.b ~~-1` = `00`). It binds at
        // the same atom tier as `~` and unary `-`, tighter than every binary
        // operator: `dc.b ~~0+1` = `02` (`(~~0)+1`), `dc.b ~~0*3` = `03`,
        // `dc.b ~~0=1` = `01`. `~~~x` is `~~` then `~` by maximal munch:
        // `dc.b ~~~0,~~~1,~~~5` = `00 00 00`.
        Tok::Punct(Punct::TildeTilde) => {
            let (inner, r) = parse_atom(rest, depth)?;
            Some((
                Expr::Unary {
                    op: UnOp::LogNot,
                    operand: Box::new(inner),
                },
                r,
            ))
        }
        Tok::Punct(Punct::LParen) => {
            let (inner, r) = parse_bp(rest, 0, depth)?;
            match r.first().map(|t| &t.tok) {
                Some(Tok::Punct(Punct::RParen)) => Some((inner, &r[1..])),
                _ => None, // unbalanced paren
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod depth_guard_tests {
    //! The AS front end had NO expression-depth guard at all (sigil lens sweep
    //! 2026-08-13, seat SAFE, finding S19). `parse_atom`'s three recursive arms
    //! (`-x`, `~x`, `(x)`) were bounded only by input size, so a syntactically
    //! PERFECT deeply-parenthesised operand overflowed the native stack and
    //! aborted the process with SIGABRT. That is not a panic — it cannot be
    //! `catch_unwind`'d, so `sigil` died with no diagnostic and no location.
    //! Measured aborting at 40,000 nesting levels.
    //!
    //! This frontend is live: it assembles `game_root.asm` on every build, and the
    //! shape is reachable from every operand and every `dc.b/w/l`.
    //!
    //! Each case parses on a child thread with a bounded stack, so the assertion is
    //! about the guard rather than about whatever stack the harness happens to give
    //! the main thread, and a regression FAILS (thread died) instead of taking the
    //! whole test binary down with it.
    use super::parse_expr;
    use crate::lexer::lex_line;
    use sigil_ir::backend::Cpu;
    use sigil_span::SourceId;

    fn parses(src: String) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();
        let h = std::thread::Builder::new()
            .stack_size(4 * 1024 * 1024)
            .spawn(move || {
                let toks = lex_line(&src, Cpu::M68000, SourceId(0), 0).expect("lex");
                let _ = tx.send(parse_expr(&toks).is_some());
            })
            .expect("spawn");
        let out = rx
            .recv_timeout(std::time::Duration::from_secs(60))
            .expect("parse did not terminate");
        h.join().expect("parser thread died — stack-overflow regression");
        out
    }

    #[test]
    fn deep_parens_are_refused_not_aborted() {
        let n = 60_000;
        assert!(
            !parses(format!("{}1{}", "(".repeat(n), ")".repeat(n))),
            "a {n}-deep parenthesised operand must be REFUSED, not aborted"
        );
    }

    #[test]
    fn deep_unary_chains_are_refused_not_aborted() {
        let n = 60_000;
        assert!(!parses(format!("{}1", "-".repeat(n))), "deep `-` chain must be refused");
        assert!(!parses(format!("{}1", "~".repeat(n))), "deep `~` chain must be refused");
    }

    /// Guard the other direction — a bound set too low would refuse real
    /// expressions. The corpus nests single digits deep.
    #[test]
    fn ordinary_nesting_still_parses() {
        for n in [1usize, 8, 64, 100] {
            assert!(
                parses(format!("{}1{}", "(".repeat(n), ")".repeat(n))),
                "{n}-deep parens are ordinary and must parse"
            );
        }
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
    fn hash_modulo_and_precedence() {
        let none = |_: &str| None;
        assert_eq!(fold("256 # 64", &none), 0);
        assert_eq!(fold("100 # 7", &none), 2);
        assert_eq!(fold("255 # 256", &none), 255);
        assert_eq!(fold("(-5) # 3", &none), -2);
        assert_eq!(fold("5 # (-3)", &none), 2);
        // `#` binds like `*`/`/` — tighter than `+` (asl-verified).
        assert_eq!(fold("7 # 5 * 2", &none), 4);
        assert_eq!(fold("5 + 7 # 2", &none), 6);
        assert_eq!(fold("7 # 2 + 5", &none), 6);
    }

    #[test]
    fn symbols_and_dollar() {
        let env = |n: &str| match n {
            "Ids_End" => Some(0x8290),
            "Ids" => Some(0x8284),
            "$" => Some(0x38),
            _ => None,
        };
        assert_eq!(fold("Ids_End - Ids", &env), 0x0C);
        assert_eq!(fold("38h - $", &env), 0); // $ bound to 0x38
    }
}
