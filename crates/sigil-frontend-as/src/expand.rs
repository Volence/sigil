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
/// The three built-in names FOLD CASE and a parameter name does not, under `-U`
/// (asl-verified, one expansion of `cm macro Pp` called `cm.w Zz`):
///
/// ```text
///    7/ 1000 : 615B 5A7A 5D20     dc.b "a[Zz] b[pp] c[PP] d[Zz] e[Zz] f[.w]"
/// ```
///
/// — source `a[Pp] b[pp] c[PP] d[allargs] e[ALLARGS] f[.attribute]`. The
/// parameter answers only to the spelling it was declared with; `allargs` and
/// `.attribute` answer to any.
pub(crate) fn substitute_frame(
    text: &str,
    attribute: Option<&str>,
    all_args: &str,
    int_label: Option<&str>,
    params: &[String],
    bound: &[String],
) -> String {
    const ATTRIBUTE: &str = ".ATTRIBUTE";
    const ALLARGS: &str = "ALLARGS";
    const LABEL: &str = "__LABEL__";

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
