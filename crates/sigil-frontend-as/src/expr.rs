//! expr: token-slice → `sigil_ir::Expr` with AS-flavoured precedence.

use crate::charset::CodePage;
use crate::nameless::{self, NamelessCounts};
use crate::token::{Punct, Tok, Token};
use sigil_ir::expr::{BinOp, UnOp};
use sigil_ir::Expr;

/// Everything an expression parse needs from the assembler's state, in one
/// place.
///
/// It started as a bare `&CodePage` and grew the nameless-label counters, which
/// is the reason it is a struct and not two arguments: both are properties of
/// WHERE IN THE PASS the statement sits, both are needed at the leaves, and
/// bundling them means adding a third cannot silently miss a call site. The
/// counters are a snapshot by value for the same reason the code page is
/// borrowed read-only — this parser stays stateless, and a reference resolves
/// against the position it was parsed at and nothing later.
#[derive(Copy, Clone)]
pub struct ExprCtx<'a> {
    /// The active `charset` translation.
    pub cs: &'a CodePage,
    /// The nameless-label counters as of this statement.
    pub nameless: NamelessCounts,
}

impl<'a> ExprCtx<'a> {
    /// A context with no nameless labels in scope.
    ///
    /// Test-only on purpose: every production caller is a statement in a pass
    /// and has real counters to hand (`eval.rs::ectx`), so a convenience that
    /// zeroes them is a way for one to lose them silently. `#[cfg(test)]` makes
    /// that a compile error rather than a wrong slot number.
    #[cfg(test)]
    pub fn plain(cs: &'a CodePage) -> Self {
        Self {
            cs,
            nameless: NamelessCounts::default(),
        }
    }
}

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

/// The most characters a string literal may carry and still be an integer:
/// asl's integer conversion holds four bytes and refuses a fifth.
const MAX_PACKED_CHARS: usize = 4;

/// AS's STRING-AS-INTEGER rule, for an EXPRESSION context only.
///
/// A string literal of one to four characters IS an integer in asl: the
/// characters are packed BIG-ENDIAN, each contributing its unsigned byte, and
/// the result is an ordinary non-negative value that every operator accepts.
/// Sonic 1 spends it as a two-character command tag. `move.w #"SW",...` writes
/// `$5357` into a child object's command field and `cmpi.w #"GO",...` reads it
/// back, so getting the packing wrong is a wrong ROM byte, not a wrong
/// diagnostic.
///
/// Every case below is asl-measured (md5 `61e672562465725a8c102288a7da9098`,
/// exit 0 quoted per probe in
/// `docs/superpowers/notes/2026-09-09-s1-expr-char-literal-and-defined.md`):
///
/// ```text
///   move.l #"A",d0        203C 0000 0041      one character, ZERO-extended
///   move.l #"AB",d0       203C 0000 4142
///   move.l #"ABC",d0      203C 0041 4243
///   move.l #"ABCD",d0     203C 4142 4344
///   move.w #"SW",d0       303C 5357           the corpus's own tag
///   move.w #"AB"+1,d0     303C 4143           an ordinary integer in arithmetic
/// ```
///
/// UNSIGNED, and measured as such rather than assumed: a high bit does not sign
/// -extend. `move.l #"\xff",d0` is `0000 00FF` (not `FFFF FFFF`), `#"\x80"+0`
/// is `0000 0080`, `#"\xff\xff"+1` is `0001 0000` (not `0000 0000`), and
/// `#("\xff\xff"<0)` is `0000 0000`. So the value is the plain base-256 reading
/// of the bytes, and the width of the target does not enter into it:
/// `move.b #"AB",d0` converts to `$4142` and then draws asl's own
/// `error #1320: range overflow`, rather than the string being refused.
///
/// `None` for an EMPTY string and for five characters or more, which is asl's
/// answer too: both draw `error #1141: expected integer, but got string` at
/// exit 2. Returning `None` makes the operand not-an-expression, so the caller
/// raises its own refusal at the same line asl refuses. The limit is four on
/// the Z80 as well as the 68000 (`ld hl,"ABCD">>16` is `21 42 41`;
/// `ld hl,"ABCDE"` is the same `#1141`), so it belongs to asl's integer type and
/// not to the target's word size.
///
/// **This rule is for EXPRESSIONS, and a `dc`-family directive is not one.** In
/// a data directive a string is a CHARACTER SEQUENCE, one element per character
/// at the directive's width, and an operator distributes over the elements
/// rather than over a packed value: asl's `dc.w "AB"` is `0041 0042`, and so is
/// `dc.w "AB"+0`. `directive_db` already consumes that shape before it reaches
/// this parser; the wider directives refuse a string operand outright rather
/// than let it arrive here and pack, because packing it would be silently wrong
/// bytes rather than a loud refusal.
///
/// **The `charset` seam**, now wired: `cs` IS the code page, and it replaces the
/// character-to-byte step alone. The big-endian packing above sits on top of
/// whatever byte a character maps to, so the two compose without either knowing
/// about the other: under `charset 'A','X',$11`, asl reads `move.w #"AB",d0` as
/// `303C 1112` and this function returns `$1112`.
///
/// The page reaches THREE sites in this front end: this function,
/// `eval.rs::directive_db`, and `lexer.rs`'s character constant, which packs
/// `'AB'` at lex time and maps each character through `cs.map_char` as it goes.
/// The count matters more than it looks: the sentence this replaced named a
/// population of TWO, it was quoted into a dispatch brief as authoritative, and
/// the third site was found only because the parcel enumerated instead of
/// inheriting. asl has a FOURTH: its wide data directives
/// distribute a string operand and translate each character (`dc.w "AB"` under
/// that same `charset` is `0011 0042`). But sigil refuses a string operand to
/// `dc.w`/`dc.l` outright (`STRING_IN_WIDE_DATA`), so that site does not exist
/// here. If it is ever implemented, it is a code-page consumer on day one.
///
/// The page is threaded as an argument rather than held: this parser stays
/// stateless, and every call site is named by the compiler instead of by a
/// reader's enumeration.
pub(crate) fn string_to_int(s: &str, cs: &CodePage) -> Option<i64> {
    let mut packed: i64 = 0;
    let mut chars = 0usize;
    for c in s.chars() {
        chars += 1;
        if chars > MAX_PACKED_CHARS {
            return None;
        }
        packed = (packed << 8) | i64::from(cs.map_char(c));
    }
    if chars == 0 {
        None
    } else {
        Some(packed)
    }
}

/// Parse a leading expression from `toks`; return it plus the unconsumed tail.
/// `None` if the head is not an expression, or if it nests past
/// [`MAX_EXPR_DEPTH`] — the same "not an expression here" answer the unbalanced-
/// paren arm already returns, so callers report a clean parse error either way.
pub fn parse_expr<'a>(toks: &'a [Token], ctx: &ExprCtx<'_>) -> Option<(Expr, &'a [Token])> {
    parse_bp(toks, 0, 0, ctx)
}

/// Binding-power ladder: higher binds tighter. It is asl's, tier for tier, and
/// it is NOT C's:
///
/// ```text
///   9  <<  >>                 tightest
///   8  &
///   7  |
///   6  !                      (bitwise xor)
///   5  *  /  #
///   4  +  -
///   3  &&
///   2  ||
///   1  =  <>  <  >  <=  >=    loosest
/// ```
///
/// Three things about it are surprising if you are reading it as C. The SHIFTS
/// and the BITWISE operators bind tighter than multiplication, so `1+1<<3` is
/// `1+(1<<3)` = 9 and `3*2|5` is `3*(2|5)` = 21. `!` is its OWN tier, looser
/// than `|`, so `3!1|2` is `3!(1|2)` = 0. And the COMPARISONS are the loosest
/// tier of all, looser than `&&` and `||`, so `A=6&&C<>3` is `(A=(6&&C))<>3`,
/// not the C reading `(A=6)&&(C<>3)`.
///
/// Every tier boundary above is measured, not inferred: the probes are in
/// `docs/superpowers/notes/2026-09-05-as-logical-precedence-probes/` and the
/// bytes are asserted in `tests/as_operator_precedence.rs`. A tier that is
/// merely PLAUSIBLE here is a silent wrong answer in a folded integer, since
/// sigil and asl both exit 0 and neither mentions the expression.
///
/// `&&` and `||` are NORMALISING logical operators, not bitwise ones: asl folds
/// `6&&3` to `1` and `4||2` to `1`, so a nonzero operand contributes only its
/// truth. That is a claim about VALUES, and it is independent of the tiers above.
///
/// `pub(crate)` because the front-end-only TYPED evaluator
/// (`eval.rs::eval_num`, which backs `int(...)`/`sin(...)` and float-valued
/// symbols) walks the same operator surface. Sharing this one ladder is what
/// keeps the two parsers from drifting: a precedence change lands in both, and
/// the typed evaluator's `BinOp` arms are exhaustive, so a NEW operator added
/// here fails to compile there until it is given a typed meaning.
pub(crate) fn infix_bp(p: Punct) -> Option<(u8, BinOp)> {
    use Punct::*;
    Some(match p {
        // The shifts are the TIGHTEST binary tier, above `*` and `/`: asl folds
        // `12/2<<1` to 3, which is `12/(2<<1)`, and `1+1<<3` to 9. They share
        // one left-associative tier with each other (`8>>1<<2`=16).
        Shl => (9, BinOp::Shl),
        Shr => (9, BinOp::Shr),
        // The bitwise tier is three SEPARATE tiers, all above `*`: `3*2&5`=0 is
        // `3*(2&5)`, `1|2&2`=3 is `1|(2&2)`, and `3!1|2`=0 is `3!(1|2)`.
        Amp => (8, BinOp::And),
        Pipe => (7, BinOp::Or),
        // `!` — AS's infix bitwise XOR (asl-verified 2026-07-04: `1!1`=0,
        // `3!1`=2, `5!3`=6; the earlier bitwise-OR reading was wrong, since the
        // only prior golden `3!4`=7 cannot tell OR from XOR). Its own tier,
        // LOOSER than `|`. Drives `__ErrorMessage`'s
        // `.__align_flag: set (((*)&1)!1)*$80`.
        Bang => (6, BinOp::Xor),
        Star => (5, BinOp::Mul),
        Slash => (5, BinOp::Div),
        // `#` infix modulo, same left-associative tier as `*`/`/` (asl-verified:
        // `7#5*2`=4, `12#5/2`=1, `5+7#2`=6). Distinct from the OPERAND-level
        // `#expr` immediate marker, which `operands.rs::classify` consumes from
        // the front of an operand group before this parser ever sees it: by the
        // time `parse_expr` runs, any remaining `#` is unambiguously infix.
        Hash => (5, BinOp::Mod),
        Plus => (4, BinOp::Add),
        Minus => (4, BinOp::Sub),
        // The logical pair binds TIGHTER than every comparison, and `&&`
        // tighter than `||`: `2=2&&1` is `2=(2&&1)`=0, and `1||0&&0` is
        // `1||(0&&0)`=1.
        AndAnd => (3, BinOp::LogAnd),
        OrOr => (2, BinOp::LogOr),
        // The comparisons are ONE left-associative tier and the loosest one:
        // `1<2=1`=1 is `(1<2)=1` and `2=1<2`=1 is `(2=1)<2`.
        Eq => (1, BinOp::Eq),
        Ne => (1, BinOp::Ne),
        Lt => (1, BinOp::Lt),
        Gt => (1, BinOp::Gt),
        Le => (1, BinOp::Le),
        Ge => (1, BinOp::Ge),
        _ => return None,
    })
}

fn parse_bp<'a>(
    toks: &'a [Token],
    min_bp: u8,
    depth: u32,
    ctx: &ExprCtx<'_>,
) -> Option<(Expr, &'a [Token])> {
    let (mut lhs, mut rest) = parse_atom(toks, depth, ctx)?;
    while let Some(Tok::Punct(p)) = rest.first().map(|t| &t.tok) {
        let (bp, op) = match infix_bp(*p) {
            Some(x) if x.0 > min_bp => x,
            _ => break,
        };
        let (rhs, r2) = parse_bp(&rest[1..], bp, depth, ctx)?;
        lhs = Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
        rest = r2;
    }
    Some((lhs, rest))
}

fn parse_atom<'a>(toks: &'a [Token], depth: u32, ctx: &ExprCtx<'_>) -> Option<(Expr, &'a [Token])> {
    if depth >= MAX_EXPR_DEPTH {
        return None;
    }
    let depth = depth + 1;
    // AS's NAMELESS TEMPORARY LABELS, in the one position where `+` and `-` can
    // be a label rather than an operator: the head of a primary expression.
    //
    // `parse_atom` runs ONLY where an operand is expected — `parse_bp` consumes
    // an infix operator itself and never calls back in with one at the head — so
    // reaching here with a `+`/`-` already means "no left-hand side". That is
    // exactly AS's own condition, and it is why this needs no lookbehind and no
    // statement-level pre-pass.
    //
    // The run rule is [`nameless::starts_atom`]'s doc comment: in a run of n
    // leading `+`/`-`, the LAST is the binary operator whenever an operand
    // follows the run, so the reference is the first n-1; with nothing to apply
    // an operator to, the reference is all n.
    //
    // The `ref_len == 0` fall-through is what keeps this feature away from
    // arithmetic that already worked. A single `-` before an operand — every
    // `-1`, `-Base`, `-(SIZE*2)` in every corpus — takes the unary-negation arm
    // below, byte for byte as before. What changes for a currently-ACCEPTED
    // expression is only `-` × m, m >= 2, before an operand: sigil folded that
    // as m nested negations, and asl reads it as `(nameless) - operand`. The
    // aeon closure contains no such run (3 files, 899 lines, measured at the
    // consuming end) and the Sonic 2 corpus's 14 are all nameless references.
    if let Some(Tok::Punct(kind @ (Punct::Plus | Punct::Minus))) = toks.first().map(|t| &t.tok) {
        let kind = *kind;
        let run = toks
            .iter()
            .take_while(|t| matches!(t.tok, Tok::Punct(Punct::Plus | Punct::Minus)))
            .count();
        let operand_follows = toks.get(run).is_some_and(|t| nameless::starts_atom(&t.tok));
        let ref_len = if operand_follows { run - 1 } else { run };
        if ref_len >= 1 {
            // A MIXED run has no reading: asl splits `+--Base` at its rightmost
            // `-`, is left with `+-`, splits that at ITS rightmost, and refuses
            // the empty right-hand side (`error: wrong number of operands`).
            // `None` here is the caller's own refusal at the same line.
            if !toks[..ref_len]
                .iter()
                .all(|t| matches!(t.tok, Tok::Punct(p) if p == kind))
            {
                return None;
            }
            let k = ref_len as u32;
            let name = match kind {
                Punct::Plus => nameless::fwd_slot(ctx.nameless.fwd + k),
                // Backward slot `bwd - k + 1`. A reference deeper than the
                // definitions behind it would underflow; naming slot 0 instead
                // gives an ordinary undefined symbol, which is the diagnostic
                // asl raises for it (`error: symbol undefined`).
                _ => nameless::bwd_slot((ctx.nameless.bwd + 1).saturating_sub(k)),
            };
            return Some((Expr::Sym(name), &toks[ref_len..]));
        }
    }
    let (head, rest) = toks.split_first()?;
    match &head.tok {
        Tok::Int(n) => Some((Expr::Int(*n), rest)),
        // A string literal in a primary-expression position is asl's packed
        // integer; see [`string_to_int`] for the rule and for why a `dc`-family
        // directive must never reach this arm.
        Tok::Str(s) => string_to_int(s, ctx.cs).map(|v| (Expr::Int(v), rest)),
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
            let (inner, r) = parse_atom(rest, depth, ctx)?;
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
            let (inner, r) = parse_atom(rest, depth, ctx)?;
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
            let (inner, r) = parse_atom(rest, depth, ctx)?;
            Some((
                Expr::Unary {
                    op: UnOp::LogNot,
                    operand: Box::new(inner),
                },
                r,
            ))
        }
        Tok::Punct(Punct::LParen) => {
            let (inner, r) = parse_bp(rest, 0, depth, ctx)?;
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
    use super::{parse_expr, ExprCtx};
    use crate::charset::CodePage;
    use crate::lexer::lex_line;
    use sigil_ir::backend::Cpu;
    use sigil_span::SourceId;

    fn parses(src: String) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();
        let h = std::thread::Builder::new()
            .stack_size(4 * 1024 * 1024)
            .spawn(move || {
                let toks = lex_line(&src, Cpu::M68000, &CodePage::identity(), SourceId(0), 0).expect("lex");
                let _ = tx.send(parse_expr(&toks, &CodePage::identity()).is_some());
            })
            .expect("spawn");
        let out = rx
            .recv_timeout(std::time::Duration::from_secs(60))
            .expect("parse did not terminate");
        h.join().expect("parser thread died, stack-overflow regression");
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
    use super::{parse_expr, ExprCtx};
    use crate::lexer::lex_line;
    use crate::charset::CodePage;
    use sigil_ir::backend::Cpu;
    use sigil_ir::expr::Fold;
    use sigil_span::SourceId;

    fn fold(src: &str, lookup: &dyn Fn(&str) -> Option<i64>) -> i64 {
        let toks = lex_line(src, Cpu::Z80, &CodePage::identity(), SourceId(0), 0).unwrap();
        let cs = CodePage::identity();
        let (e, rest) = parse_expr(&toks, &ExprCtx::plain(&cs)).unwrap();
        assert!(rest.is_empty(), "unconsumed tokens: {rest:?}");
        match e.fold(lookup) {
            Fold::Value(v) => v,
            Fold::Poison => panic!("poison"),
            Fold::Fault(f) => panic!("{f}"),
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
