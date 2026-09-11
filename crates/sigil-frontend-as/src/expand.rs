//! Token-level substitution helpers for macro/function expansion, split out of
//! `eval.rs` to keep that module focused on the pass/dispatch core.

use crate::token::{Punct, Tok, Token};
use sigil_span::Span;

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
///
/// Deliberately identifier-only: it does NOT try to keep two adjacent PUNCT
/// tokens from merging into a multi-char operator (e.g. a bare `Lt` then `Gt`
/// re-lexing as `Ne`). Inserting a space there would defeat the whole point —
/// a macro argument like `<<` embedded in a `%<…>` string must render as `<<`,
/// not `< <`, to stay byte-exact. Adjacent bare comparison/shift operators do
/// not occur in any Aeon macro argument (the lexer folds `<>`/`<<`/… into single
/// tokens at their source), so the hazard is unreachable; both ROM gates prove
/// the identifier-only rule byte-neutral for the real corpus.
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
        Punct::LBracket => "[",
        Punct::RBracket => "]",
        Punct::OrOr => "||",
        Punct::AndAnd => "&&",
        Punct::Comma => ",",
        Punct::Colon => ":",
        Punct::Hash => "#",
        Punct::ColonEq => ":=",
        Punct::Bang => "!",
        Punct::Tilde => "~",
        Punct::TildeTilde => "~~",
    }
}

/// Substitute one macro expansion's bindings into a body line's text in a
/// SINGLE left-to-right pass, so no substituted text is ever rescanned.
///
/// The single pass is the whole point, and it is what AS's storage model gives
/// for free: AS resolves a body's parameter references to `\001\00N`
/// placeholders when the macro is CAPTURED, so text pasted in at expansion time
/// is inert — it contains no placeholders and cannot acquire any. Substituting
/// by successive whole-text replaces does not have that property: a value
/// pasted for one name is still in the buffer when the next name is scanned for,
/// so an argument whose text happens to spell a parameter name gets rewritten a
/// second time. asl `-U`, `mm macro pp,qq` called `mm qq,zz`, emitting
/// `"E<ALLARGS>"`:
///
/// ```text
///   11/ 1000 : (MACRO)                  mm    qq,zz
///   11/ 1000 : 453C 7171 2C7A              dc.b    "E<qq,zz>"
/// ```
///
/// The argument text is `qq,zz` — the `qq` stays the identifier the caller
/// wrote, even though `qq` is also this expansion's second parameter.
///
/// At each source position the candidates are tried in AS's own precedence —
/// `.ATTRIBUTE`, then `ALLARGS`, then `__LABEL__`, then the parameters in
/// declaration order — and the first that matches consumes its source text.
/// Every one of them obeys the SAME boundary rule ([`boundary_ok`]); an empty
/// parameter name never matches.
///
/// The built-in names FOLD CASE and a parameter name does not, under `-U`
/// (asl-verified, one expansion of `cm macro Pp` called `cm.w Zz`):
///
/// ```text
///    7/ 1000 : 615B 5A7A 5D20     dc.b "a[Zz] b[pp] c[PP] d[Zz] e[Zz] f[.w]"
/// ```
///
/// — source `a[Pp] b[pp] c[PP] d[allargs] e[ALLARGS] f[.attribute]`. The
/// parameter answers only to the spelling it was declared with; `allargs` and
/// `.attribute` answer to any.
///
/// `ARGCOUNT` is a substitution too, not a symbol — the macro listing shows the
/// DIGITS pasted into the body line, exactly as it shows `ALLARGS`'s text. It
/// folds case and obeys the same boundary rule, and it is tried AFTER the
/// parameters because a parameter declared with that name WINS (asl-verified,
/// `ac2 macro pp` called `ac2 7,8` and `ac3 macro ARGCOUNT` called `ac3 zz`,
/// probe `p9.asm`):
///
/// ```text
///    9/ 1002 : 315B 325D 2032     dc.b "1[2] 2[xARGCOUNTx] 3[_2_] 4[2] 5[2]"
///   15/ 1027 : 5B7A 7A5D          dc.b "[zz]"
/// ```
pub(crate) fn substitute_frame(
    text: &str,
    attribute: Option<&str>,
    all_args: &str,
    int_label: Option<&str>,
    params: &[String],
    bound: &[String],
    arg_count: i64,
) -> String {
    const ATTRIBUTE: &str = ".ATTRIBUTE";
    const ALLARGS: &str = "ALLARGS";
    const LABEL: &str = "__LABEL__";
    const ARGCOUNT: &str = "ARGCOUNT";
    let arg_count_text = arg_count.to_string();

    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    'outer: while i < bytes.len() {
        let rest = &text[i..];
        for (name, value) in [
            (ATTRIBUTE, attribute),
            (ALLARGS, Some(all_args)),
            (LABEL, int_label),
        ] {
            let Some(value) = value else { continue };
            if !folded_match(rest, name) || !boundary_ok(text, i, name) {
                continue;
            }
            out.push_str(value);
            i += name.len();
            continue 'outer;
        }
        for (p, a) in params.iter().zip(bound.iter()) {
            if p.is_empty() || !rest.starts_with(p.as_str()) || !boundary_ok(text, i, p) {
                continue;
            }
            out.push_str(a);
            i += p.len();
            continue 'outer;
        }
        // AFTER the parameters: a parameter declared `ARGCOUNT` shadows the
        // built-in (probe `p9.asm` case 9b). Reaching here means no parameter
        // claimed this position, so the built-in is free to.
        if folded_match(rest, ARGCOUNT) && boundary_ok(text, i, ARGCOUNT) {
            out.push_str(&arg_count_text);
            i += ARGCOUNT.len();
            continue 'outer;
        }
        let c = rest.chars().next().unwrap_or_default();
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Substitute ONE name's text into a body line, by the same single left-to-right
/// pass and the same [`boundary_ok`] rule [`substitute_frame`] uses, and with a
/// parameter's case sensitivity rather than a built-in's folding.
///
/// This is `irp`/`irpc`'s loop variable. It is deliberately NOT
/// [`substitute_frame`] with a one-entry parameter list: the loop body of a
/// macro-nested loop has ALREADY been frame-substituted once where the loop was
/// entered, so re-offering `ALLARGS`/`.ATTRIBUTE`/`__LABEL__`/`ARGCOUNT` here
/// would substitute a second time into text that is now the CALLER's, which is
/// exactly the rescanning [`substitute_frame`]'s single pass exists to prevent.
///
/// Case-sensitive, asl-verified under `-U` (probe `p7.asm` case 7f,
/// `irpc Cv,"AB"` over `dc.b "<Cv><cv>"`):
///
/// ```text
///   33/ 102A : 3C41 3E3C 6376 3E     dc.b "<A><cv>"
/// ```
pub(crate) fn substitute_name(text: &str, name: &str, value: &str) -> String {
    if name.is_empty() {
        return text.to_string();
    }
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    while i < bytes.len() {
        let rest = &text[i..];
        if rest.starts_with(name) && boundary_ok(text, i, name) {
            out.push_str(value);
            i += name.len();
            continue;
        }
        let c = rest.chars().next().unwrap_or_default();
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Whether `rest` begins with `name`, comparing ASCII case-insensitively. The
/// three built-in substitution names are AS KEYWORDS, and a keyword folds even
/// under `-U` — which is why `{intlabel}` declares the capture and `__label__`
/// reads it back.
fn folded_match(rest: &str, name: &str) -> bool {
    rest.len() >= name.len() && rest.as_bytes()[..name.len()].eq_ignore_ascii_case(name.as_bytes())
}

/// AS's boundary rule for a substituted name, measured rather than assumed.
///
/// A candidate at byte `i` is rejected when an ALPHANUMERIC character abuts an
/// edge of it that could continue an identifier. Two halves, each independent:
///
/// * the character BEFORE, when `name` starts with an identifier character;
/// * the character AFTER, when `name` ends with one.
///
/// `_` is an identifier character but NOT alphanumeric, so it never blocks. That
/// asymmetry is the whole rule, and it is what makes the corpus's `{INTLABEL}`
/// idiom work at all: `__LABEL___End` composes because the trailing `_` does not
/// block, while `xx__LABEL__` stays verbatim because `x` does. One expansion of
/// `pm macro pp` called `pm Zz`, and one of `lm macro {INTLABEL}` under `Qq:`,
/// give the same nine answers:
///
/// ```text
///   10/ 1000 : 315B 5F5A 7A5D  dc.b "1[_Zz] 2[1pp] 3[Xpp] 4[.Zz] 5[ppX] 6[pp1] 7[Zz_] 8[(Zz)] 9[__Zz__]"
///   11/ 1042 : 315B 5F51 715D  dc.b "1[_Qq] 2[1__LABEL__] 3[X__LABEL__] 4[.Qq] 5[__LABEL__X] 6[__LABEL__1] 7[Qq_] 8[(Qq)]"
/// ```
///
/// `ALLARGS` answers identically (`xALLARGSx` verbatim, `_ALLARGS_` → `_Zz_`).
/// `.ATTRIBUTE` differs ONLY through the per-edge test: it begins with `.`,
/// which cannot continue an identifier, so no leading check applies and the
/// glued-mnemonic use survives — `move.ATTRIBUTE` → `move.w` and
/// `x.ATTRIBUTE` → `x.w`, while `.ATTRIBUTEx` stays verbatim:
///
/// ```text
///    8/ 1002 : 505B 6D6F 7665  dc.b "P[move.w] Q[x.w] R[.ATTRIBUTEx]"
/// ```
fn boundary_ok(text: &str, i: usize, name: &str) -> bool {
    if name.chars().next().is_some_and(is_ident_char)
        && text[..i].chars().next_back().is_some_and(char::is_alphanumeric)
    {
        return false;
    }
    if name.chars().next_back().is_some_and(is_ident_char)
        && text[i + name.len()..]
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
    {
        return false;
    }
    true
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

/// How many arguments asl counts in the user-function call whose `(` is
/// `toks[lparen]` and whose `)` is `toks[rparen]`, given the groups
/// [`split_call_args`] returned for it.
///
/// asl counts argument TEXT: every comma starts an argument, but the text after
/// the LAST comma (or the whole text, with no comma) counts only when it is not
/// empty, and a blank is not empty. Measured, `x` a one-parameter function and
/// `y`/`z` two and three (probes `e1_func*`, exit status quoted):
///
/// ```text
///   f()      #1490 wrong numbers of function arguments   (0 arguments)
///   f( )     01                                          (1: " ")
///   f(,1)    01                                          (2: "", "1")
///   f(1,)    #1490                                       (1: "1")
///   f(,)     #1490                                       (1: "")
///   f(1,,2)  03                                          (3: "1", "", "2")
/// ```
///
/// The lexer has dropped the blanks, so "empty" is read off the spans: the last
/// group is empty text only when the token before the `)` ends where the `)`
/// begins. Every empty argument that does count is the value 0 (`()`), which is
/// why this count, and not the group count, is what decides arity.
pub(crate) fn asl_call_arg_count(
    toks: &[Token],
    lparen: usize,
    rparen: usize,
    groups: &[Vec<Token>],
) -> usize {
    let n = groups.len();
    let last_empty = groups.last().is_some_and(Vec::is_empty);
    let (Some(before), Some(close)) = (rparen.checked_sub(1).and_then(|i| toks.get(i)), toks.get(rparen)) else {
        return n;
    };
    let adjacent = before.span.source == close.span.source && before.span.end == close.span.start;
    if last_empty && adjacent && rparen > lparen {
        n - 1
    } else {
        n
    }
}

/// Where to blame a diagnostic about ONE item of a comma-separated list: the
/// item's own first token, falling back to the directive's span when the group
/// is empty.
///
/// A data directive holds many items on one line, so blaming every one of them
/// at the directive renders two mistakes as two byte-identical lines. `dc.b
/// Big, Big` reported `probe.asm(3): error: operand 74565 out of range
/// -128..=255` twice, and nothing in either line said which item it was about.
/// Since [`SourceMap::label`](sigil_span::SourceMap::label) carries a column,
/// the item's own span is what makes the two lines different.
///
/// Only DIAGNOSTIC spans move. The span an item's bytes are emitted under stays
/// the directive's, because that one is the line-to-offset attribution the
/// listing and the fixup records are built from, and it is not a claim about
/// where a mistake is.
pub(crate) fn item_span(g: &[Token], directive: Span) -> Span {
    group_span(g).unwrap_or(directive)
}

/// Split a token slice on top-level (non-parenthesised, non-bracketed) commas.
///
/// A `[...]` duplicate-operand count is one group's own prefix, so a comma
/// inside it belongs to the count expression and not to the operand list:
/// `dc.b [f(1,2)]$FF` is ONE operand, exactly as `dc.b (f(1,2))` is.
pub(crate) fn split_top_commas(toks: &[Token]) -> Vec<&[Token]> {
    let mut groups = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, t) in toks.iter().enumerate() {
        match t.tok {
            Tok::Punct(Punct::LParen) | Tok::Punct(Punct::LBracket) => depth += 1,
            Tok::Punct(Punct::RParen) | Tok::Punct(Punct::RBracket) => depth -= 1,
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

/// The byte ranges of a macro call's ARGUMENTS within `operand`, the text that
/// follows the macro's name on the invocation line, each trimmed of blanks.
///
/// AS arguments are TEXT: asl cuts the operand at commas and pastes each piece
/// as written. A word no lexer rule reads (`2p.bin`, `1up`, `a@b`, `\x41`) is an
/// argument like any other, and a number keeps its spelling: `$10` stays `$10`,
/// `007` stays `007`, `%101` stays `%101`, and a tab inside an argument stays a
/// tab. asl `-U`, exit 0 (probes `m5*`), the second column read off the
/// expansion's own listing line:
///
/// ```text
///   pal Special Stage 1 2p.bin       dc.b "Special Stage 1 2p.bin"
///   m   aa  ,  bb  ,cc   ; comment   (aa)(bb)(cc)   ALLARGS aa,bb,cc   ARGCOUNT 3
///   m (1,2),3   and   m [1,2],3      one argument each side of the outer comma
///   m "a,b",c   m "a;b",c   m 'a,b',c   a literal keeps its comma and its `;`
///   m af',bb                         `af'` is a register name, not a quote
///   m ,bb,                           ()(bb)()       ARGCOUNT 3
/// ```
///
/// A comma splits only outside parentheses, brackets and literals, and a `;`
/// outside a literal starts the comment. Each argument is trimmed of blanks at
/// both ends and keeps the blanks inside it. An operand with no text has no
/// arguments at all (`ARGCOUNT` 0), while `m ,` has two empty ones.
pub(crate) fn split_macro_args(operand: &str) -> Vec<(usize, usize)> {
    let b = operand.as_bytes();
    let mut pieces = Vec::new();
    let (mut depth, mut start, mut i) = (0i32, 0usize, 0usize);
    let mut end = b.len();
    while i < b.len() {
        if let Some(next) = skip_literal(b, i) {
            i = next;
            continue;
        }
        match b[i] {
            b';' => {
                end = i;
                break;
            }
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b',' if depth == 0 => {
                pieces.push(trim_blanks(b, start, i));
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let last = trim_blanks(b, start, end);
    if pieces.is_empty() && last.0 == last.1 {
        return pieces;
    }
    pieces.push(last);
    pieces
}

/// Where a macro argument's KEYWORD separator sits: the offset of the first `=`
/// in the argument's text outside parentheses, brackets and literals, or `None`
/// for a positional argument.
///
/// asl decides this on the argument's text, before it means anything: whatever
/// stands left of that `=`, trimmed, is the keyword's NAME whether or not it is
/// an identifier, a parameter, or even non-empty, and the trimmed text right of
/// it is the value. asl `-U`, `m macro px,py,pz` over `dc.b px,py,pz` (probes
/// `k3.asm`-`k7.asm`):
///
/// ```text
///    7/       0 : (MACRO)              	m	1,zz=2,3
///    7/       0 :                             dc.b    1,,
///    8/       3 : (MACRO)              	m	1,2=3,4
///    8/       3 :                             dc.b    1,,
///    7/       0 : (MACRO)              	m	1,(2=3),4
///    7/       0 : 0100 04                     dc.b    1,(2=3),4
///    8/       0 : (MACRO)              	m	1,"py=2",3
///    8/       0 : 0170 793D 3203              dc.b    1,"py=2",3
///   10/       5 : (MACRO)              	m	1,2<>3,4
///   10/       5 : 0101 04                     dc.b    1,2<>3,4
/// ```
///
/// So `zz=2` and `2=3` split (and bind nothing, which is why both expansions
/// show two empty fields); a parenthesised or quoted `=` does not split; and
/// `<>`, carrying no `=` at all, is an ordinary expression. And over `message
/// "(pa)(pb)(pc)"` (probes `m5k_*`): `m pb==5` binds `pb` to `=5`, because the
/// FIRST `=` splits; `m pb= 5` and `m pb =5` both bind `5`; and `m 2<=3` is
/// `#1811 keyword argument not defined in macro`, its name being `2<`.
#[allow(clippy::tabs_in_doc_comments)]
pub(crate) fn keyword_eq_offset(arg: &str) -> Option<usize> {
    let b = arg.as_bytes();
    let (mut depth, mut i) = (0i32, 0usize);
    while i < b.len() {
        if let Some(next) = skip_literal(b, i) {
            i = next;
            continue;
        }
        match b[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'=' if depth == 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// If a string or character literal opens at `b[i]`, the index just past its
/// closing quote (or the end of the text, unterminated). A `'` right after an
/// identifier character is part of that word, the Z80's `af'`, not an opener.
fn skip_literal(b: &[u8], i: usize) -> Option<usize> {
    let opens = match b[i] {
        b'"' => true,
        b'\'' => i == 0 || !(b[i - 1].is_ascii_alphanumeric() || matches!(b[i - 1], b'_' | b'.' | b'\'')),
        _ => false,
    };
    opens.then(|| crate::escape::literal_end(b, i).map_or(b.len(), |close| close + 1))
}

/// `[s, e)` with blanks, and the line's own end, removed from both ends.
fn trim_blanks(b: &[u8], mut s: usize, mut e: usize) -> (usize, usize) {
    while s < e && matches!(b[s], b' ' | b'\t' | b'\r' | b'\n') {
        s += 1;
    }
    while e > s && matches!(b[e - 1], b' ' | b'\t' | b'\r' | b'\n') {
        e -= 1;
    }
    (s, e)
}

/// The span covering one argument group, or `None` when the group is empty.
///
/// A diagnostic about an argument must point AT that argument: a macro call
/// carries several, and a whole-line span cannot say which one was refused.
pub(crate) fn group_span(g: &[Token]) -> Option<Span> {
    let first = g.first()?.span;
    let last = g.last()?.span;
    Some(Span { source: first.source, start: first.start, end: last.end })
}
