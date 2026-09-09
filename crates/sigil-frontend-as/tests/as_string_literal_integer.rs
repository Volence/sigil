//! A string literal in an EXPRESSION is an integer: one to four characters,
//! packed big-endian, each contributing its unsigned byte.
//!
//! Sonic 1 spends this on a two-character command tag. `_incObj/82, 83 SBZ
//! Eggman Cutscene and Crumbling Floor.asm` writes `move.w #"SW",…` into a
//! child object's command field and reads it back with `cmpi.w #"GO",…`, six
//! sites in all, so the packing is a ROM byte and not a diagnostic. A wrong
//! packing removes all six complaints and improves every count while writing
//! the wrong word, which is why the six sites below are asserted as BYTES
//! against the reference listing rather than merely assembling.
//!
//! ## Provenance
//!
//! Every expected value here comes from
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, invoked `-xx -n -q -A -L -U -i .`, with
//! the exit status checked per probe and quoted at each test. The other `asl`
//! build in this workspace (md5 `0dee1f98e6480a4783d27ffd8b90896f`) was not run
//! for any value here; it answers differently on every run for an operand it
//! declines, and the banner cannot tell the two apart.
//!
//! A zero exit is necessary and not sufficient: for an operand it declines to
//! value, this build substitutes THE LAST VALUE IT COMPUTED, silently. The
//! packing values below were re-run with the probe lines in two different
//! orders and with an unrelated `move.l #$1234,d0` moved above and below them
//! (probes `sp.asm`, `sq.asm`, both exit 0). `"SW"` reads `5357` and `"GO"`
//! reads `474F` in every position and neither ever echoes the `1234`, so these
//! are answers rather than artifacts.
//!
//! ## The rule, and its edges
//!
//! ```text
//!   move.l #"A",d0        203C 0000 0041     probe sa, exit 0
//!   move.l #"AB",d0       203C 0000 4142
//!   move.l #"ABC",d0      203C 0041 4243
//!   move.l #"ABCD",d0     203C 4142 4344
//!   move.w #"AB"+1,d0     303C 4143          an ordinary integer in arithmetic
//!   move.l #"\xff",d0     203C 0000 00FF     probe sd, exit 0: UNSIGNED
//!   move.l #"\xff\xff"+1  203C 0001 0000     probe sl, exit 0: no wrap, no sign
//!   move.l #"",d0         error #1141        probe sc, exit 2
//!   move.l #"ABCDE",d0    error #1141        probe sb, exit 2
//!   move.b #"AB",d0       error #1320        probe sf, exit 2: converts, THEN
//!                                            range-checks, so the conversion is
//!                                            independent of the target width
//! ```
//!
//! The four-character limit is asl's integer type and not the target's word
//! size: on the Z80 `ld hl,"ABCD">>16` is `21 42 41` and `ld hl,"ABCDE"` draws
//! the same `#1141` (probes `sm`, `ss`).
//!
//! ## One measured divergence, stated rather than modelled
//!
//! At exactly four characters with every bit set, asl's overflow domain does
//! something this evaluator does not reproduce: `move.l #"\xff\xff\xff\xff"+1,d0`
//! is `0000 0000` at exit 0, while the identically-valued
//! `move.l #$FFFFFFFF+1,d0` is `error #1320: range overflow` at exit 2 (probes
//! `so`, `sh`). It is not the substitution artifact: with `move.l #$1234,d0`
//! immediately above it the answer is still `0000 0000` and not `1234`, so asl
//! computed it. sigil treats the packed value as the plain integer 4294967295,
//! so `+1` is 4294967296 and the immediate's own range check refuses it, which
//! agrees with asl on the literal spelling and is louder than asl on the string
//! one. No site in s1disasm, s2disasm, skdisasm or aeon writes a four-character
//! string literal at all; the longest is two.

use sigil_frontend_as::{assemble_root_located, Options};

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
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

fn bytes(body: &str) -> Vec<u8> {
    match assemble(body) {
        Ok(b) => b,
        Err(d) => panic!("expected bytes, got diagnostics: {d:?}"),
    }
}

fn diags(body: &str) -> Vec<String> {
    match assemble(body) {
        Ok(b) => panic!("expected a refusal, got bytes: {b:02X?}"),
        Err(d) => d,
    }
}

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";

/// THE SIX CORPUS SITES, operand for operand, in file order: lines 133, 163,
/// 185, 255, 293 and 312 of `_incObj/82, 83 SBZ Eggman Cutscene and Crumbling
/// Floor.asm`, with that file's own `SEgg_ChildCmd equ obSubtype` and
/// `_Constants.asm`'s `obSubtype equ $28`.
///
/// asl, probe `sites.asm`, exit 0, `1 pass`, `0 errors`:
///
/// ```text
///        8/       0 : 317C 5357 0028             move.w  #"SW",SEgg_ChildCmd(a0)
///        9/       6 : 337C 474F 0028             move.w  #"GO",SEgg_ChildCmd(a1)
///       10/       C : 0C69 5357 0028             cmpi.w  #"SW",SEgg_ChildCmd(a1)
///       11/      12 : 0C68 474F 0028             cmpi.w  #"GO",SEgg_ChildCmd(a0)
///       12/      18 : 337C 474F 0028             move.w  #"GO",SEgg_ChildCmd(a1)
///       13/      1E : 0C68 474F 0028             cmpi.w  #"GO",SEgg_ChildCmd(a0)
/// ```
///
/// A little-endian packing would write `5753`/`4F47` here and every one of the
/// six corpus complaints would still have gone away.
#[test]
fn the_six_corpus_sites_match_the_reference_listing() {
    let src = format!(
        "{HEAD}\
obSubtype:\tequ $28\n\
SEgg_ChildCmd:\tequ obSubtype\n\
\tmove.w\t#\"SW\",SEgg_ChildCmd(a0)\n\
\tmove.w\t#\"GO\",SEgg_ChildCmd(a1)\n\
\tcmpi.w\t#\"SW\",SEgg_ChildCmd(a1)\n\
\tcmpi.w\t#\"GO\",SEgg_ChildCmd(a0)\n\
\tmove.w\t#\"GO\",SEgg_ChildCmd(a1)\n\
\tcmpi.w\t#\"GO\",SEgg_ChildCmd(a0)\n\
\tend\n"
    );
    assert_eq!(
        bytes(&src),
        vec![
            0x31, 0x7C, 0x53, 0x57, 0x00, 0x28,
            0x33, 0x7C, 0x47, 0x4F, 0x00, 0x28,
            0x0C, 0x69, 0x53, 0x57, 0x00, 0x28,
            0x0C, 0x68, 0x47, 0x4F, 0x00, 0x28,
            0x33, 0x7C, 0x47, 0x4F, 0x00, 0x28,
            0x0C, 0x68, 0x47, 0x4F, 0x00, 0x28,
        ]
    );
}

/// One to four characters, packed big-endian and ZERO-extended to the target.
/// asl, probe `sa.asm`, exit 0, `1 pass`, `0 errors`, the four `move.l` lines
/// quoted in the module header.
#[test]
fn one_to_four_characters_pack_big_endian() {
    let src = format!(
        "{HEAD}\tmove.l #\"A\",d0\n\tmove.l #\"AB\",d0\n\
         \tmove.l #\"ABC\",d0\n\tmove.l #\"ABCD\",d0\n\tend\n"
    );
    assert_eq!(
        bytes(&src),
        vec![
            0x20, 0x3C, 0x00, 0x00, 0x00, 0x41,
            0x20, 0x3C, 0x00, 0x00, 0x41, 0x42,
            0x20, 0x3C, 0x00, 0x41, 0x42, 0x43,
            0x20, 0x3C, 0x41, 0x42, 0x43, 0x44,
        ]
    );
}

/// The packed value is an ordinary integer: it composes with every operator.
/// asl, probe `sa.asm`, exit 0: `move.w #"AB"+1,d0` is `303C 4143` and
/// `move.l #"AB"*2,d0` is `203C 0000 8284`.
#[test]
fn a_packed_string_is_an_ordinary_integer_in_arithmetic() {
    let src = format!("{HEAD}\tmove.w #\"AB\"+1,d0\n\tmove.l #\"AB\"*2,d0\n\tend\n");
    assert_eq!(
        bytes(&src),
        vec![0x30, 0x3C, 0x41, 0x43, 0x20, 0x3C, 0x00, 0x00, 0x82, 0x84]
    );
}

/// A high bit is a VALUE, not a sign. asl, probe `sd.asm`, exit 0:
/// `move.l #"\xff",d0` is `203C 0000 00FF` and `move.l #"\xff\xff",d0` is
/// `203C 0000 FFFF`; probe `sl.asm`, exit 0: `move.l #"\xff\xff"+1,d0` is
/// `203C 0001 0000` and `move.l #("\xff\xff"<0),d0` is `203C 0000 0000`.
///
/// A sign-extending packer would write `FFFF FFFF`, `0000 0000` and `0000 0001`
/// for these three, so this test can come out three other ways.
#[test]
fn a_high_bit_is_a_value_not_a_sign() {
    let src = format!(
        "{HEAD}\tmove.l #\"\u{ff}\",d0\n\tmove.l #\"\u{ff}\u{ff}\",d0\n\
         \tmove.l #\"\u{ff}\u{ff}\"+1,d0\n\tmove.l #(\"\u{ff}\u{ff}\"<0),d0\n\tend\n"
    );
    assert_eq!(
        bytes(&src),
        vec![
            0x20, 0x3C, 0x00, 0x00, 0x00, 0xFF,
            0x20, 0x3C, 0x00, 0x00, 0xFF, 0xFF,
            0x20, 0x3C, 0x00, 0x01, 0x00, 0x00,
            0x20, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ]
    );
}

/// An empty string and a fifth character are both refused, as they are by asl:
/// `error #1141: expected integer, but got string`, exit 2 (probes `sc.asm`,
/// `sb.asm`). The wording differs; the refusal is the point, and it is at the
/// same line.
#[test]
fn an_empty_or_over_long_string_is_refused() {
    for operand in ["\"\"", "\"ABCDE\""] {
        let src = format!("{HEAD}\tmove.l #{operand},d0\n\tend\n");
        let d = diags(&src);
        assert!(
            d.iter().any(|m| m.contains("bad immediate expression")),
            "{operand} must be refused, got {d:?}"
        );
    }
}

/// The conversion does not consult the target's width: a two-character string
/// converts to `$4142` and then draws the ORDINARY out-of-range complaint, which
/// is asl's `error #1320: range overflow` at exit 2 (probe `sf.asm`). A packer
/// that refused on width grounds would say "bad byte expression" instead.
#[test]
fn a_two_character_string_in_a_byte_is_a_range_complaint_not_a_parse_one() {
    let src = format!("{HEAD}\tmove.b #\"AB\",d0\n\tend\n");
    let d = diags(&src);
    assert!(
        d.iter().any(|m| m.contains("out of range")),
        "expected a range complaint, got {d:?}"
    );
}

/// A `dc.b`/`db` string stays a CHARACTER SEQUENCE and is NOT packed. asl,
/// probe `sr.asm`, exit 0: `dc.b "AB"` is `4142`, one byte per character.
///
/// This is the regression that matters most in the other direction: 56 lines of
/// `sonic.asm` alone write `dc.b "text"`, and packing one would be silently
/// wrong bytes across the whole corpus.
#[test]
fn a_byte_directive_string_is_still_a_character_sequence() {
    let src = format!("{HEAD}\tdc.b \"AB\"\n\tdc.b \"Hello\"\n\tend\n");
    assert_eq!(
        bytes(&src),
        vec![0x41, 0x42, 0x48, 0x65, 0x6C, 0x6C, 0x6F]
    );
}

/// A data directive WIDER than a byte refuses a string operand rather than
/// packing it.
///
/// asl reads it as a character sequence at the directive's width, and an
/// operator distributes over the elements rather than over a packed value
/// (probe `sr.asm`, exit 0): `dc.w "AB"` is `0041 0042`, and `dc.w "AB"+0` is
/// `0041 0042` too. The expression parser would pack both to `4142`, so these
/// must not reach it. The character-sequence form is not implemented at these
/// widths and no line in s1disasm, s2disasm, skdisasm or aeon writes one.
#[test]
fn a_wide_data_directive_refuses_a_string_rather_than_packing_it() {
    for line in ["\tdc.w \"AB\"\n", "\tdc.w \"AB\"+0\n", "\tdc.l \"ABCD\"\n"] {
        let src = format!("{HEAD}{line}\tend\n");
        let d = diags(&src);
        assert!(
            d.iter().any(|m| m.contains("string operand in a data directive")),
            "{line:?} must be refused, got {d:?}"
        );
    }
}
