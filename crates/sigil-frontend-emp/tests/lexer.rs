use sigil_frontend_emp::lexer::{lex, Tok};
use sigil_span::SourceId;

fn toks(src: &str) -> Vec<Tok> {
    let (tokens, errs) = lex(src, SourceId(0));
    assert!(errs.is_empty(), "lex errors: {errs:?}");
    tokens.into_iter().map(|t| t.tok).collect()
}

#[test]
fn idents_punct_newlines() {
    assert_eq!(
        toks("proc wait (a0: *Sst) {\n}"),
        vec![
            Tok::ident("proc"), Tok::ident("wait"),
            Tok::LParen, Tok::ident("a0"), Tok::Colon, Tok::Star, Tok::ident("Sst"), Tok::RParen,
            Tok::LBrace, Tok::Newline, Tok::RBrace, Tok::Eof,
        ]
    );
}

#[test]
fn multichar_operators() {
    assert_eq!(
        toks("a == b != c <= d >= e << f >> g ++ h && i || j -> k .. l"),
        vec![
            Tok::ident("a"), Tok::EqEq, Tok::ident("b"), Tok::Ne, Tok::ident("c"),
            Tok::Le, Tok::ident("d"), Tok::Ge, Tok::ident("e"), Tok::Shl, Tok::ident("f"),
            Tok::Shr, Tok::ident("g"), Tok::PlusPlus, Tok::ident("h"), Tok::AndAnd,
            Tok::ident("i"), Tok::OrOr, Tok::ident("j"), Tok::Arrow, Tok::ident("k"),
            Tok::DotDot, Tok::ident("l"), Tok::Eof,
        ]
    );
}

#[test]
fn fat_arrow_is_one_token() {
    // `=>` (match arms) is a single token, distinct from `=` and `>`.
    assert_eq!(toks("=>"), vec![Tok::FatArrow, Tok::Eof]);
    assert_eq!(toks("Pat => body"), vec![Tok::ident("Pat"), Tok::FatArrow, Tok::ident("body"), Tok::Eof]);
    assert_eq!(toks("= >"), vec![Tok::Eq, Tok::Gt, Tok::Eof]);
}

#[test]
fn pipe_variants_disambiguate() {
    // `|>` is one token, distinct from `||` and a bare `|`.
    assert_eq!(toks("|>"), vec![Tok::PipeGt, Tok::Eof]);
    assert_eq!(toks("||"), vec![Tok::OrOr, Tok::Eof]);
    assert_eq!(toks("|"), vec![Tok::Pipe, Tok::Eof]);
    assert_eq!(toks("a |> b"), vec![Tok::ident("a"), Tok::PipeGt, Tok::ident("b"), Tok::Eof]);
    assert_eq!(toks("a | b"), vec![Tok::ident("a"), Tok::Pipe, Tok::ident("b"), Tok::Eof]);
    assert_eq!(toks("a || b"), vec![Tok::ident("a"), Tok::OrOr, Tok::ident("b"), Tok::Eof]);
}

#[test]
fn dot_is_its_own_token() {
    // `move.b` and `.draw:` both decompose; the parser reassembles.
    assert_eq!(
        toks("move.b .draw:"),
        vec![Tok::ident("move"), Tok::Dot, Tok::ident("b"),
             Tok::Dot, Tok::ident("draw"), Tok::Colon, Tok::Eof]
    );
}

#[test]
fn spans_are_byte_offsets() {
    let (tokens, _) = lex("ab cd", SourceId(3));
    assert_eq!(tokens[1].span.start, 3);
    assert_eq!(tokens[1].span.end, 5);
    assert_eq!(tokens[1].span.source, SourceId(3));
}

#[test]
fn numeric_literals() {
    assert_eq!(
        toks("64 $60 0b1010 3.5"),
        vec![Tok::Int(64), Tok::Int(0x60), Tok::Int(0b1010), Tok::Float(3.5), Tok::Eof]
    );
}

#[test]
fn hex_is_case_insensitive_in_digits() {
    assert_eq!(toks("$FFFF8000 $ff"), vec![Tok::Int(0xFFFF_8000), Tok::Int(0xFF), Tok::Eof]);
}

#[test]
fn float_requires_digit_after_dot() {
    // `0..256` is Int DotDot Int, not a malformed float
    assert_eq!(toks("0..256"), vec![Tok::Int(0), Tok::DotDot, Tok::Int(256), Tok::Eof]);
}

#[test]
fn string_literals_with_escapes() {
    assert_eq!(
        toks(r#""hi \"there\"\n""#),
        vec![Tok::Str("hi \"there\"\n".to_string()), Tok::Eof]
    );
}

#[test]
fn bad_dollar_is_an_error() {
    let (_, errs) = lex("$zz", SourceId(0));
    assert_eq!(errs.len(), 1);
}

#[test]
fn non_ascii_is_an_error_not_a_panic() {
    let (toks_out, errs) = lex("a €b", SourceId(0));
    assert_eq!(errs.len(), 1);
    // lexing continues after the bad char
    assert!(toks_out.iter().any(|t| t.tok == Tok::ident("b")));
}

// ---- Z80 shadow-register apostrophe (B1) --------------------------------

#[test]
fn shadow_register_apostrophe_is_part_of_ident() {
    // `af'`, `bc'`, ... (Z80 shadow registers) lex as a SINGLE identifier,
    // apostrophe included.
    assert_eq!(toks("af'"), vec![Tok::ident("af'"), Tok::Eof]);
    assert_eq!(toks("bc'"), vec![Tok::ident("bc'"), Tok::Eof]);
    assert_eq!(
        toks("ex af, af'"),
        vec![Tok::ident("ex"), Tok::ident("af"), Tok::Comma, Tok::ident("af'"), Tok::Eof]
    );
}

#[test]
fn plain_ident_without_apostrophe_is_unaffected() {
    // `exx` and other apostrophe-free idents keep lexing exactly as before.
    assert_eq!(toks("exx"), vec![Tok::ident("exx"), Tok::Eof]);
    assert_eq!(toks("af"), vec![Tok::ident("af"), Tok::Eof]);
}

#[test]
fn only_one_trailing_apostrophe_is_absorbed() {
    // A single trailing `'` joins the ident; a second `'` is a stray char that
    // errors (and must not panic).
    let (_, errs) = lex("af''", SourceId(0));
    assert_eq!(errs.len(), 1, "second apostrophe should be a stray-char error");
}

#[test]
fn bare_apostrophe_is_an_error_not_a_panic() {
    // A `'` not following an identifier is an unexpected character, not a crash.
    let (_, errs) = lex("'", SourceId(0));
    assert_eq!(errs.len(), 1);
}
