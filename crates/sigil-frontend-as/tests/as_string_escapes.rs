//! AS escape sequences inside string and character literals, and how they meet
//! `charset`.
//!
//! Sonic 2 writes its 35 font tables as `charset 'B',"\4\8\xC\4\x10..."`, and a
//! build that does not process escapes maps each character to a character of the
//! UNPROCESSED text: `dc.b "EMERALD HILL"` came out `5C 35 5C 5C 31 32 32` (`\`,
//! `5`, `\`, `\`, `1`, `2`, `2`) where asl writes `22 2A 22 2F 1E 29 21`, over
//! 2,176 bytes, exit 0, with no diagnostic. Every test here is aimed at that
//! failure's shapes, including the ones the corpus cannot show.
//!
//! # Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
//! `Macro Assembler 1.42 Beta [Bld 212]`, md5
//! `61e672562465725a8c102288a7da9098`, invoked `-xx -n -q -A -L -U -i .`, one
//! construct per probe file. **Every expected byte below was read out of a run
//! that exited 0**; every refusal is that build's non-zero answer, read for
//! accept-or-refuse only. Probe sources, the full summary and the checker are
//! in `docs/superpowers/notes/2026-09-11-as-string-escapes/`.
//!
//! # What a half-fix looks like, and which test goes red
//!
//! | half-fix | red here |
//! |---|---|
//! | escapes in `dc.b` strings but not in `charset` targets | `a_charset_target_takes_its_escapes_and_stores_them_raw`, `sonic_2_s_charset_backslash_h_line_assembles` |
//! | escapes in `charset` targets but not in `dc.b` strings | `every_escape_form_asl_accepts_emits_asl_s_bytes_in_a_data_string` |
//! | one numeric form in the wrong base (`\NNN` read as octal, `\0NNN` as decimal) | `every_escape_form_asl_accepts_emits_asl_s_bytes_in_a_data_string` |
//! | an escaped character emitted raw, bypassing the live `charset` | `an_escaped_character_goes_through_the_live_code_page` |
//! | character constants left unescaped (`charset '\H'` refused) | `sonic_2_s_charset_backslash_h_line_assembles`, `a_character_constant_takes_the_same_escapes` |
//!
//! The code-page row is the one only a NON-identity page can catch. With the
//! identity page live, "the escaped character goes through the page" and "the
//! escape is a raw byte" produce the same bytes, so every other test in this
//! file stays green under that half-fix.

use sigil_frontend_as::{assemble_root_located, Options};

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const HEAD_Z80: &str = "\tcpu z80\n\torg 0\n";

fn assemble_with(head: &str, body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("{head}{body}\n\tend\n")).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok(sigil_link::flatten(&linked, 0x00).unwrap())
        }
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    assemble_with(HEAD, body)
}

/// Assemble every `(body, asl's bytes)` case and report ALL the mismatches at
/// once, so a regression names each form it broke rather than the first.
fn check_all(head: &str, cases: &[(&str, &[u8])]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(body, want)| match assemble_with(head, body) {
            Ok(got) if got == *want => None,
            other => Some(format!("  {body:?}\n    asl   {want:02X?}\n    sigil {other:02X?}")),
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} cases differ from asl:\n{}",
        wrong.len(),
        cases.len(),
        wrong.join("\n")
    );
}

/// Assemble every body and require a refusal; report every one that came back
/// as bytes.
fn check_refused(cases: &[&str], must_mention: Option<&str>) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|body| match assemble(body) {
            Err(d) if must_mention.is_none_or(|m| d.iter().any(|x| x.contains(m))) => None,
            other => Some(format!("  {body:?} -> {other:02X?}")),
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} refusals were not refused{}:\n{}",
        wrong.len(),
        cases.len(),
        must_mention.map(|m| format!(" with a diagnostic naming `{m}`")).unwrap_or_default(),
        wrong.join("\n")
    );
}

// ---------------------------------------------------------------------------
// The grammar, in a data directive on the identity page.
// ---------------------------------------------------------------------------

/// Every escape form asl accepts, each written alone in `dc.b "..."`.
///
/// The rows are the probe matrix's exit-0 rows verbatim. Note the digit counts
/// and the base split, which is where a plausible guess goes wrong: `\x` takes
/// at most two hex digits and zero is allowed; `\0` starts OCTAL with at most
/// three more digits; any other leading digit is DECIMAL with at most three.
#[test]
fn every_escape_form_asl_accepts_emits_asl_s_bytes_in_a_data_string() {
    let cases: &[(&str, &[u8])] = &[
        (r#" dc.b "\a""#, &[0x07]),
        (r#" dc.b "\A""#, &[0x07]),
        (r#" dc.b "\b""#, &[0x08]),
        (r#" dc.b "\B""#, &[0x08]),
        (r#" dc.b "\e""#, &[0x1B]),
        (r#" dc.b "\E""#, &[0x1B]),
        (r#" dc.b "\h""#, &[0x27]),
        (r#" dc.b "\H""#, &[0x27]),
        (r#" dc.b "\i""#, &[0x22]),
        (r#" dc.b "\I""#, &[0x22]),
        (r#" dc.b "\n""#, &[0x0A]),
        (r#" dc.b "\N""#, &[0x0A]),
        (r#" dc.b "\r""#, &[0x0D]),
        (r#" dc.b "\R""#, &[0x0D]),
        (r#" dc.b "\t""#, &[0x09]),
        (r#" dc.b "\T""#, &[0x09]),
        (r#" dc.b "\\""#, &[0x5C]),
        (r#" dc.b "\"""#, &[0x22]),
        (r#" dc.b "\'""#, &[0x27]),
        (r#" dc.b "\x41""#, &[0x41]),
        (r#" dc.b "\X41""#, &[0x41]),
        (r#" dc.b "\x4""#, &[0x04]),
        (r#" dc.b "\x0""#, &[0x00]),
        (r#" dc.b "\x""#, &[0x00]),
        (r#" dc.b "\X""#, &[0x00]),
        (r#" dc.b "\xG""#, &[0x00, 0x47]),
        (r#" dc.b "\x4G""#, &[0x04, 0x47]),
        (r#" dc.b "\x4Z""#, &[0x04, 0x5A]),
        (r#" dc.b "\x414""#, &[0x41, 0x34]),
        (r#" dc.b "\x100""#, &[0x10, 0x30]),
        (r#" dc.b "\x041""#, &[0x04, 0x31]),
        (r#" dc.b "\x41B""#, &[0x41, 0x42]),
        (r#" dc.b "\x7f""#, &[0x7F]),
        (r#" dc.b "\xff""#, &[0xFF]),
        (r#" dc.b "\xFF""#, &[0xFF]),
        (r#" dc.b "\xab""#, &[0xAB]),
        (r#" dc.b "\xAb""#, &[0xAB]),
        (r#" dc.b "\0""#, &[0x00]),
        (r#" dc.b "\00""#, &[0x00]),
        (r#" dc.b "\0000""#, &[0x00]),
        (r#" dc.b "\07""#, &[0x07]),
        (r#" dc.b "\010""#, &[0x08]),
        (r#" dc.b "\012""#, &[0x0A]),
        (r#" dc.b "\0123""#, &[0x53]),
        (r#" dc.b "\0377""#, &[0xFF]),
        (r#" dc.b "\00012""#, &[0x01, 0x32]),
        (r#" dc.b "\01234""#, &[0x53, 0x34]),
        (r#" dc.b "\0a""#, &[0x00, 0x61]),
        (r#" dc.b "\0x41""#, &[0x00, 0x78, 0x34, 0x31]),
        (r#" dc.b "\1""#, &[0x01]),
        (r#" dc.b "\7""#, &[0x07]),
        (r#" dc.b "\8""#, &[0x08]),
        (r#" dc.b "\9""#, &[0x09]),
        (r#" dc.b "\12""#, &[0x0C]),
        (r#" dc.b "\18""#, &[0x12]),
        (r#" dc.b "\99""#, &[0x63]),
        (r#" dc.b "\123""#, &[0x7B]),
        (r#" dc.b "\255""#, &[0xFF]),
        (r#" dc.b "\1234""#, &[0x7B, 0x34]),
        (r#" dc.b "\12x""#, &[0x0C, 0x78]),
        (r#" dc.b "\65B""#, &[0x41, 0x42]),
        (r#" dc.b "\1a""#, &[0x01, 0x61]),
        (r#" dc.b "\x3B\2\4\6\8\xA\xC\xE\x10""#, &[0x3B, 0x02, 0x04, 0x06, 0x08, 0x0A, 0x0C, 0x0E, 0x10]),
    ];
    check_all(HEAD, cases);
}

/// Every escape form asl refuses. Letters: all but `a b e h i n r t x`, in
/// either case (`error #2010: invalid escape sequence`). Numbers: a value above
/// 255, or an 8 or 9 after `\0` (`error #1320: range overflow`).
#[test]
fn every_escape_form_asl_refuses_is_refused_out_loud() {
    let mut cases: Vec<String> = Vec::new();
    for c in "cdfgjklmopqsuvwyzCDFGJKLMOPQSUVWYZ".chars() {
        cases.push(format!(" dc.b \"\\{c}\""));
    }
    for p in r#"? .,;$#@%&()*+-/:<=>[]^_`|~}!"#.chars() {
        cases.push(format!(" dc.b \"\\{p}\""));
    }
    for n in ["256", "999", "08", "09", "018", "0400", "0777", "08A"] {
        cases.push(format!(" dc.b \"\\{n}\""));
    }
    let refs: Vec<&str> = cases.iter().map(String::as_str).collect();
    check_refused(&refs, Some("escape sequence"));
}

/// A backslash that ends the literal escapes the closing quote, so the literal
/// never closes; an unterminated `\{` has no expression. asl refuses both.
#[test]
fn a_backslash_with_nothing_after_it_is_refused() {
    check_refused(&[r#" dc.b "ab\""#, r#" dc.b "ab\",$11"#, "n equ 5\n dc.b \"a\\{n\""], None);
}

/// An escaped quote does not close the literal, and the `;` after it is a
/// character, not a comment. Before this was handled the lexer closed the
/// string at `\"` and read the rest of the line as a comment: `dc.b "a\";b"`
/// assembled to `61 5C` at exit 0.
#[test]
fn an_escaped_quote_does_not_end_the_literal() {
    check_all(
        HEAD,
        &[
            (r#" dc.b "a\"b""#, &[0x61, 0x22, 0x62]),
            (r#" dc.b "a\";b""#, &[0x61, 0x22, 0x3B, 0x62]),
            (r#" dc.b "a\",b",$11"#, &[0x61, 0x22, 0x2C, 0x62, 0x11]),
            (r#" dc.b "it's""#, &[0x69, 0x74, 0x27, 0x73]),
            (r#" dc.w '"'"#, &[0x00, 0x22]),
        ],
    );
}

// ---------------------------------------------------------------------------
// The same grammar in every other context a literal appears in.
// ---------------------------------------------------------------------------

/// A `'...'` character constant is packed at lex time, a separate site from the
/// string path, and it takes the same escapes.
#[test]
fn a_character_constant_takes_the_same_escapes() {
    check_all(
        HEAD,
        &[
            (r" dc.w '\x41'", &[0x00, 0x41]),
            (r" dc.w '\65'", &[0x00, 0x41]),
            (r" dc.w '\H'", &[0x00, 0x27]),
            (r" dc.w '\h'", &[0x00, 0x27]),
            (r" dc.w '\''", &[0x00, 0x27]),
            (r" dc.w '\\'", &[0x00, 0x5C]),
            (r#" dc.w '\"'"#, &[0x00, 0x22]),
            (r" dc.w '\n'", &[0x00, 0x0A]),
            (r" dc.l '\x41\x42\x43\x44'", &[0x41, 0x42, 0x43, 0x44]),
            (r" dc.l 'A\x42C\68'", &[0x41, 0x42, 0x43, 0x44]),
            (r" move.w #'\x41',d0", &[0x30, 0x3C, 0x00, 0x41]),
            (r" dc.b '\x41'", &[0x41]),
        ],
    );
}

/// A string in an EXPRESSION is packed big-endian, and the count of at most
/// four characters is a count of characters AFTER escapes: `"\x41\x42"` is two.
#[test]
fn a_string_in_an_expression_counts_and_packs_escaped_characters() {
    check_all(
        HEAD,
        &[
            (r#" move.l #"\x41\x42",d0"#, &[0x20, 0x3C, 0x00, 0x00, 0x41, 0x42]),
            (r#" move.l #"\x41\x42\x43\x44",d0"#, &[0x20, 0x3C, 0x41, 0x42, 0x43, 0x44]),
            (r#" move.w #"\H",d0"#, &[0x30, 0x3C, 0x00, 0x27]),
            (r#" move.w #"\x41"+1,d0"#, &[0x30, 0x3C, 0x00, 0x42]),
        ],
    );
}

/// The string builtins, comparisons and string symbols see the VALUE.
#[test]
fn string_builtins_and_string_symbols_see_the_escaped_value() {
    check_all(
        HEAD,
        &[
            (r#" dc.b strlen("\x41\66\\")"#, &[0x03]),
            (r#" dc.b "\x41"="A""#, &[0x01]),
            (r#" dc.b substr("\x41\x42\x43",1,1)"#, &[0x42]),
            (r#" dc.b val("\x31\x32")"#, &[0x0C]),
            (r#" dc.b lowstring("\x41")"#, &[0x61]),
            ("s := \"\\x41\\66\"\n dc.b s\n dc.b strlen(s)", &[0x41, 0x42, 0x02]),
            ("s equ \"\\x41\\66\"\n dc.b s", &[0x41, 0x42]),
            ("s := substr(\"\\x41\\x42\\x43\",1,0)\n dc.b s", &[0x42, 0x43]),
            (" irpc c,\"\\x41B\"\n dc.b \"c\"\n endr", &[0x41, 0x42]),
        ],
    );
}

/// A macro argument is text, so a string argument reaches the body in its
/// source form and is unescaped once, where the body uses it. A value quoted
/// back into a literal must round-trip: `a\b"c` has a backslash and a quote.
#[test]
fn a_string_macro_argument_is_unescaped_once_where_it_is_used() {
    check_all(
        HEAD,
        &[
            ("m macro a\n dc.b a\n endm\n m \"\\x41\\66\"", &[0x41, 0x42]),
            ("mq macro pa,pb\n dc.b pa\n dc.b pb\n endm\n mq \"\\\"x,y\",$11", &[0x22, 0x78, 0x2C, 0x79, 0x11]),
            ("mq macro pa\n dc.b pa\n endm\n mq \"a\\\";b\"", &[0x61, 0x22, 0x3B, 0x62]),
            (
                "mq macro pa\n dc.b pa\n dc.b strlen(pa)\n endm\n mq \"a\\\\b\\\"c\"",
                &[0x61, 0x5C, 0x62, 0x22, 0x63, 0x05],
            ),
        ],
    );
}

/// The Z80 front end shares the lexer and the data path.
#[test]
fn the_z80_data_directive_and_character_constant_take_the_same_escapes() {
    check_all(
        HEAD_Z80,
        &[(" db \"\\x41\\66\\H\"", &[0x41, 0x42, 0x27]), (r" ld a,'\x41'", &[0x3E, 0x41])],
    );
}

/// `\{expr}` is part of the same scan: escapes and interpolations are read left
/// to right in one pass, so `\\{n}` is a backslash followed by the text `{n}`.
#[test]
fn an_interpolation_and_the_escapes_around_it_are_one_scan() {
    check_all(
        HEAD,
        &[
            ("n equ 5\n dc.b \"\\x41\\{n}\"", &[0x41, 0x35]),
            ("n equ 5\ns := \"\\\\{n}\"\n dc.b s", &[0x5C, 0x7B, 0x6E, 0x7D]),
            ("n equ 5\ns := \"\\x41\\{n}\\x42\"\n dc.b s\n dc.b strlen(s)", &[0x41, 0x35, 0x42, 0x03]),
            ("n equ $12\n charset 'A',\"\\{n}\"\n dc.b \"AB\"", &[0x31, 0x32]),
            (" dc.b \"\\{later}\"\nlater equ 7", &[0x37]),
        ],
    );
}

/// An interpolation with no value is refused in a data directive rather than
/// written out as its source text (asl: `error #1010: symbol undefined`).
#[test]
fn an_interpolation_with_no_value_is_refused_in_data() {
    check_refused(&[" dc.b \"\\{nope}\""], Some("nope"));
}

/// Each context refuses an invalid escape, as asl does, rather than writing
/// the backslash and the letter.
#[test]
fn an_invalid_escape_is_refused_in_every_context() {
    check_refused(
        &[
            r" dc.w '\c'",
            r#" move.w #"\c",d0"#,
            r#" charset 'A',"\c""#,
            r#" message "\c""#,
            r#"s := "\c""#,
            r#" move.w #"\256",d0"#,
        ],
        None,
    );
}

/// asl judges an escape only where the literal is evaluated: inside an `if 0`
/// branch and in a macro that is never called, an invalid escape draws nothing
/// (both exit 0).
#[test]
fn an_escape_is_judged_only_where_the_literal_is_evaluated() {
    check_all(
        HEAD,
        &[
            (" if 0\n dc.b \"\\c\"\n endif\n dc.b $EE", &[0xEE]),
            (" if 0\n dc.w '\\c'\n endif\n dc.b $EE", &[0xEE]),
            ("mz macro\n dc.b \"\\c\"\n endm\n dc.b $EE", &[0xEE]),
        ],
    );
}

/// `message` and `warning` text is a string like any other: asl prints
/// `mAB'\z` for `message "m\x41\66\H\\z"`, and `\{n}|5` for
/// `message "\\{n}|\{n}"`.
#[test]
fn message_and_warning_text_take_the_escapes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    let src = format!(
        "{HEAD}n equ 5\n message \"m\\x41\\66\\H\\\\z\"\n message \"\\\\{{n}}|\\{{n}}\"\n warning \"w\\x41\\66\\H\\\\z\"\n dc.b $EE\n\tend\n"
    );
    std::fs::write(&path, src).expect("write probe");
    let a = sigil_frontend_as::assemble_root_located_warned(&path, &Options::default())
        .unwrap_or_else(|f| panic!("expected success, got {:?}", f.diags));
    assert_eq!(a.messages, vec![r"mAB'\z".to_string(), r"\{n}|5".to_string()]);
    let warnings: Vec<&str> = a.warnings.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(warnings, vec![r"[as.warning] wAB'\z"]);
}

// ---------------------------------------------------------------------------
// charset.
// ---------------------------------------------------------------------------

/// The two-operand `charset` STRING target takes its escapes and stores the
/// resulting characters RAW. The count is of characters after escapes:
/// `charset $FD,"\x10\11\x12"` is three entries and fits; its twelve source
/// characters would run past `$FF`.
#[test]
fn a_charset_target_takes_its_escapes_and_stores_them_raw() {
    check_all(
        HEAD,
        &[
            (" charset 'A',\"\\x10\\11\\x12\"\n dc.b \"ABC\"", &[0x10, 0x0B, 0x12]),
            (" charset 'A',\"\\x10\\x11\\x12\"\n dc.b \"ABC\"\n charset\n dc.b \"ABC\"", &[0x10, 0x11, 0x12, 0x41, 0x42, 0x43]),
            (" charset 'A',\"\\\\\\\"\"\n dc.b \"AB\"", &[0x5C, 0x22]),
            (" charset $FD,\"\\x10\\11\\x12\"\n dc.b $FD", &[0xFD]),
        ],
    );
}

/// Sonic 2's own two `charset '\H',...` lines (`s2.asm` 14480 and 14606), in the
/// census probe's setting: a character constant as the SOURCE index, which the
/// front end refused as `charset operand 23644 out of range`.
#[test]
fn sonic_2_s_charset_backslash_h_line_assembles() {
    check_all(
        HEAD,
        &[
            (" charset '\\H',\"\\x39\\x37\\x38\"\n dc.b \"'()\"", &[0x39, 0x37, 0x38]),
            (
                " charset '@',\"\\x3B\\2\\4\\6\\8\\xA\\xC\\xE\\x10\\x12\\x13\\x15\\x17\\x19\\x1B\\x1D\\x1F\\x21\\x23\\x25\\x27\\x29\\x2B\\x2D\\x2F\\x31\\x33\"\n charset '\\H',\"\\x39\\x37\\x38\"\n dc.b \"@ABHIJ\"\n charset\n dc.b \"HIJ\"",
                &[0x3B, 0x02, 0x04, 0x10, 0x12, 0x13, 0x48, 0x49, 0x4A],
            ),
        ],
    );
}

/// THE charset interaction, on a NON-identity page. asl processes the escape
/// first and then translates the character it yields through the live page,
/// in every context: `dc.b "A\x41\65"` is `11 11 11`, not the `11 41 41` a raw
/// byte would give. A character constant used as a `charset` SOURCE is
/// translated too: under `$27 -> $55`, `charset '\H',$99` remaps index `$55`
/// (`dc.b "'U"` is `55 99`), not index `$27`.
#[test]
fn an_escaped_character_goes_through_the_live_code_page() {
    const PG: &str = " charset $41,$11\n charset $27,$55\n charset $5C,$66\n charset $07,$77\n charset $0A,$AA\n charset $22,$BB\n";
    let body = |s: &str| format!("{PG}{s}");
    let cases = [
        (body(" dc.b \"A\\x41\\65\""), vec![0x11, 0x11, 0x11]),
        (body(" dc.b \"'\\H\""), vec![0x55, 0x55]),
        (body(" dc.b \"\\\\\""), vec![0x66]),
        (body(" dc.b \"\\A\""), vec![0x77]),
        (body(" dc.b \"\\n\""), vec![0xAA]),
        (body(" dc.b \"\\\"\""), vec![0xBB]),
        (body(" dc.w '\\x41'"), vec![0x00, 0x11]),
        (body(" dc.w '\\H'"), vec![0x00, 0x55]),
        (body(" dc.w 'A'"), vec![0x00, 0x11]),
        (body(" move.w #\"\\x41\",d0"), vec![0x30, 0x3C, 0x00, 0x11]),
        (body(" charset '\\H',$99\n dc.b \"'U\""), vec![0x55, 0x99]),
        (body(" charset $50,\"\\x41A\"\n dc.b \"PQ\""), vec![0x41, 0x41]),
        (body("m macro a\n dc.b a\n endm\n m \"\\x41\""), vec![0x11]),
        (body("s := \"\\x41\"\n dc.b s"), vec![0x11]),
        (" charset $41,$11\n dc.b \"\\0101\"".to_string(), vec![0x11]),
        (" charset $41,$11\n dc.b \"\\x41\"=\"A\"".to_string(), vec![0x01]),
        (" charset $41,$11\n dc.b strlen(\"\\x41\\x41\")".to_string(), vec![0x02]),
    ];
    let owned: Vec<(&str, &[u8])> = cases.iter().map(|(b, w)| (b.as_str(), w.as_slice())).collect();
    check_all(HEAD, &owned);
    check_all(
        HEAD_Z80,
        &[(" charset 41h,11h\n db \"A\\x41\\65\"\n ld a,'\\x41'", &[0x11, 0x11, 0x11, 0x3E, 0x11])],
    );
}
