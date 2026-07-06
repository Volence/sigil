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
fn string_literal_null_escape() {
    // Lexical gaps, Task 4 part 3: `\0` in a string literal lexes to a literal
    // NUL byte, so authors can write an explicit terminator themselves
    // (`"HELLO\0"`) — no implicit trailing 0 is ever emitted for them.
    assert_eq!(toks(r#""HELLO\0""#), vec![Tok::Str("HELLO\0".to_string()), Tok::Eof]);
    // Consistent with the char-literal escape set (`'\0'` already lexes to 0).
    assert_eq!(toks(r#""a\0b""#), vec![Tok::Str("a\0b".to_string()), Tok::Eof]);
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

// ---- binary `%` literals (lexical gaps, Task 1) -------------------------

#[test]
fn binary_percent_literals() {
    assert_eq!(toks("%1010"), vec![Tok::Int(0b1010), Tok::Eof]);
    assert_eq!(toks("%0"), vec![Tok::Int(0), Tok::Eof]);
    assert_eq!(toks("%1"), vec![Tok::Int(1), Tok::Eof]);
    assert_eq!(toks("%10100101"), vec![Tok::Int(165), Tok::Eof]);
    assert_eq!(toks("%11111111"), vec![Tok::Int(255), Tok::Eof]);
}

#[test]
fn percent_stays_modulo_when_not_immediately_followed_by_binary_digit() {
    // Spaced modulo still works: `7 % 3` is Int Percent Int.
    assert_eq!(toks("7 % 3"), vec![Tok::Int(7), Tok::Percent, Tok::Int(3), Tok::Eof]);
    // `%` followed by a non-binary digit is modulo, then a separate Int.
    assert_eq!(toks("%2"), vec![Tok::Percent, Tok::Int(2), Tok::Eof]);
    // `%` followed by whitespace is modulo, whitespace is skipped as usual.
    assert_eq!(toks("% 1010"), vec![Tok::Percent, Tok::Int(1010), Tok::Eof]);
    // `%` followed by a letter is modulo, then a separate identifier.
    assert_eq!(toks("%a"), vec![Tok::Percent, Tok::ident("a"), Tok::Eof]);
    // A bare trailing `%` at EOF stays modulo.
    assert_eq!(toks("%"), vec![Tok::Percent, Tok::Eof]);
}

#[test]
fn binary_run_stops_at_first_non_binary_digit() {
    // maximal munch: the binary run ends at the first char that is not `0`/`1`,
    // so `%1012` is Int(0b101) followed by a separate Int(2).
    assert_eq!(toks("%1012"), vec![Tok::Int(0b101), Tok::Int(2), Tok::Eof]);
    // the `0b`-prefixed path cuts off identically.
    assert_eq!(toks("0b1012"), vec![Tok::Int(0b101), Tok::Int(2), Tok::Eof]);
}

#[test]
fn binary_percent_span_covers_percent_through_last_digit() {
    let (tokens, _) = lex("%1010", SourceId(0));
    assert_eq!(tokens[0].span.start, 0);
    assert_eq!(tokens[0].span.end, 5);
}

#[test]
fn binary_percent_out_of_range_is_an_error() {
    // 65 binary digits overflow i64's range, like the `0b`-prefixed path.
    let src = format!("%{}", "1".repeat(65));
    let (_, errs) = lex(&src, SourceId(0));
    assert_eq!(errs.len(), 1);
    assert!(errs[0].message.contains("out of range"), "was {:?}", errs[0].message);
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

// ---- char 'A' literals (raw ASCII) — lexical gaps, Task 3 ---------------

#[test]
fn char_literals_are_raw_ascii_ints() {
    assert_eq!(toks("'A'"), vec![Tok::Int(65), Tok::Eof]);
    assert_eq!(toks("'a'"), vec![Tok::Int(97), Tok::Eof]);
    assert_eq!(toks("'0'"), vec![Tok::Int(48), Tok::Eof]);
    assert_eq!(toks("' '"), vec![Tok::Int(32), Tok::Eof]);
    assert_eq!(toks("'~'"), vec![Tok::Int(126), Tok::Eof]);
}

#[test]
fn char_literal_flows_like_a_plain_int_in_context() {
    assert_eq!(
        toks("byte('A')"),
        vec![Tok::ident("byte"), Tok::LParen, Tok::Int(65), Tok::RParen, Tok::Eof]
    );
}

#[test]
fn char_literal_escapes() {
    assert_eq!(toks(r"'\n'"), vec![Tok::Int(10), Tok::Eof]);
    assert_eq!(toks(r"'\t'"), vec![Tok::Int(9), Tok::Eof]);
    assert_eq!(toks(r"'\\'"), vec![Tok::Int(92), Tok::Eof]);
    assert_eq!(toks(r"'\''"), vec![Tok::Int(39), Tok::Eof]);
    assert_eq!(toks(r"'\0'"), vec![Tok::Int(0), Tok::Eof]);
}

#[test]
fn char_literal_span_covers_quotes() {
    let (tokens, _) = lex("'A'", SourceId(0));
    assert_eq!(tokens[0].span.start, 0);
    assert_eq!(tokens[0].span.end, 3);
}

#[test]
fn empty_char_literal_is_an_error() {
    let (_, errs) = lex("''", SourceId(0));
    assert_eq!(errs.len(), 1);
    assert!(errs[0].message.contains("empty char literal"), "was {:?}", errs[0].message);
}

#[test]
fn multi_char_literal_is_an_error() {
    let (_, errs) = lex("'AB'", SourceId(0));
    assert_eq!(errs.len(), 1);
    assert!(
        errs[0].message.contains("single character") && errs[0].message.contains('2'),
        "was {:?}",
        errs[0].message
    );
}

#[test]
fn non_ascii_char_literal_is_an_error() {
    let (_, errs) = lex("'é'", SourceId(0));
    assert_eq!(errs.len(), 1);
    assert!(
        errs[0].message.contains("ASCII") || errs[0].message.to_lowercase().contains("ascii"),
        "was {:?}",
        errs[0].message
    );
}

#[test]
fn unterminated_char_literal_before_newline_is_an_error() {
    let (_, errs) = lex("'A\nfoo", SourceId(0));
    assert_eq!(errs.len(), 1);
    assert!(errs[0].message.contains("unterminated"), "was {:?}", errs[0].message);
    // lexing resyncs: the newline and `foo` still lex normally afterward.
    let (tokens, _) = lex("'A\nfoo", SourceId(0));
    assert!(tokens.iter().any(|t| t.tok == Tok::ident("foo")));
}

#[test]
fn unterminated_char_literal_at_eof_is_an_error() {
    let (_, errs) = lex("'A", SourceId(0));
    assert_eq!(errs.len(), 1);
    assert!(errs[0].message.contains("unterminated"), "was {:?}", errs[0].message);
}

#[test]
fn unknown_escape_in_char_literal_is_an_error() {
    let (_, errs) = lex(r"'\q'", SourceId(0));
    assert_eq!(errs.len(), 1);
    assert!(errs[0].message.contains("unknown escape"), "was {:?}", errs[0].message);
}

#[test]
fn shadow_register_apostrophe_still_wins_over_char_literal() {
    // `af'`/`bc'`/`de'`/`hl'` must still lex as single idents — the char-literal
    // arm only ever fires on a `'` the ident arm did NOT already absorb.
    assert_eq!(toks("af'"), vec![Tok::ident("af'"), Tok::Eof]);
    assert_eq!(toks("bc'"), vec![Tok::ident("bc'"), Tok::Eof]);
    assert_eq!(toks("de'"), vec![Tok::ident("de'"), Tok::Eof]);
    assert_eq!(toks("hl'"), vec![Tok::ident("hl'"), Tok::Eof]);
    // A second apostrophe (`af''`) still errors (whatever the exact message).
    let (_, errs) = lex("af''", SourceId(0));
    assert_eq!(errs.len(), 1, "second apostrophe should still be an error");
}
