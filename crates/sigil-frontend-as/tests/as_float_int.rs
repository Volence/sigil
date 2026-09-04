//! F1 — floating point literals, arithmetic, and `INT()` in the AS front end.
//!
//! Every expectation below is a byte column copied out of an asl listing, not a
//! reading of the semantics. The oracle is asl 1.42 Beta [Bld 212]
//! (`x86_64-unknown-linux`, md5 `61e672562465725a8c102288a7da9098` — the copy
//! Sonic 1's own `build.lua` invokes), run with Sonic 1's own flags
//! `-xx -n -q -A -L -U -E -i .`. The probe sources are committed under
//! `.f1probe/`; `.f1probe/cmp.sh <file.asm>` runs both assemblers on one input.
//!
//! WHY THIS EXISTS. `s1.sounddriver.asm` lines 1796 and 2052 — the bodies of
//! `MakeFMFrequenciesOctave` and `MakePSGFrequencies` — carried 166 of Sonic
//! 1's 313 front-end diagnostics between them, and they are the only front-end
//! cause that also owns retail ROM bytes: `FM_Notes` at `$72790` (192 bytes)
//! and `PSGFrequencies` at `$729CE` (138 bytes). Those 330 bytes are the real
//! gate; these tests are the unit-scale statement of the same thing, so a
//! regression names itself instead of surfacing as a ROM diff.

use sigil_frontend_as::{assemble, Options};

/// Assemble a 68000 fragment at `phase 0` and return its bytes.
fn bytes(body: &str) -> Vec<u8> {
    let src = format!("\tcpu 68000\n\tphase 0\n{body}");
    let module = assemble(&src, &Options::default()).expect("assemble");
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// Assemble and require a refusal.
fn refused(body: &str) -> Vec<String> {
    let src = format!("\tcpu 68000\n\tphase 0\n{body}");
    match assemble(&src, &Options::default()) {
        Ok(_) => panic!("expected a refusal for:\n{src}"),
        Err(d) => d.into_iter().map(|d| d.message).collect(),
    }
}

/// S1's `MacroSetup.asm:218` + `s1.sounddriver.asm:1792`, verbatim.
const S1_FM_PRELUDE: &str = "\
FM_Sample_Rate = 53267
roundFloatToInteger function float,INT(float+0.5)
MakeFMFrequency function frequency,roundFloatToInteger(frequency*1024*1024*2/FM_Sample_Rate)
";

/// S1's `MacroSetup.asm:219` + `s1.sounddriver.asm:2048`. `min` is a USER
/// function, not an asl builtin — asl answers `error #1860: unknown function
/// MIN` for a `min(` it did not define (probe `.f1probe/f0.asm(11)`), so the
/// corpus supplies its own, and the `roundFloatToInteger` inside it must have
/// collapsed to an INTEGER before `!`/`&`/`<` ever see it.
const S1_PSG_PRELUDE: &str = "\
PSG_Sample_Rate = 3546895
roundFloatToInteger function float,INT(float+0.5)
min function a,b,b!((a!b)&(-(a<b)))
MakePSGFrequency function frequency,min($3FF,roundFloatToInteger(PSG_Sample_Rate/(frequency*2)))
";

/// `FM_Notes`' base octave, the line that carried 144 of the 166.
///
/// asl listing, `.f1probe/f4.asm` (the `irp` body expanded twelve times, with
/// `octave` = 1 as `MakeFMFrequenciesOctave 1` passes it):
///
/// ```text
/// asl bytes: 0a 5e 0a 84 0a ab 0a d3 0a fe 0b 2d 0b 5c 0b 8f 0b c5 0b ff 0c 3c 0c 7c
/// ```
///
/// and with `octave` = 0 the same twelve minus `$800`, which is what sits at
/// `$72790` in the retail cartridge (`02 5e 02 84 02 ab …`).
#[test]
fn fm_notes_base_octave_matches_asl() {
    let got = bytes(&format!(
        "{S1_FM_PRELUDE}\
\tirp op, 15.39, 16.35, 17.34, 18.36, 19.45, 20.64, 21.84, 23.13, 24.51, 25.98, 27.53, 29.15
\tdc.w MakeFMFrequency(op)+1*$800
\tendm
"
    ));
    assert_eq!(
        got,
        vec![
            0x0a, 0x5e, 0x0a, 0x84, 0x0a, 0xab, 0x0a, 0xd3, 0x0a, 0xfe, 0x0b, 0x2d, 0x0b, 0x5c,
            0x0b, 0x8f, 0x0b, 0xc5, 0x0b, 0xff, 0x0c, 0x3c, 0x0c, 0x7c,
        ],
        "FM_Notes base octave must match asl's listing bytes"
    );
}

/// `PSGFrequencies`, the line that carried the other 22 — and the one that
/// proves the float result reaches an INTEGER-ONLY expression cleanly, since
/// `min`'s body is built from `!`, `&`, `<` and unary `-`, all of which asl
/// refuses on a float (`error #1134`, probe `.f1probe/f3.asm(18-20)`).
///
/// Five of the corpus's own frequencies, including both clamping ends. asl
/// listing (`.f1probe/f4.asm`): `03 ff 03 ff 00 08 00 fe 01 ac`. The first two
/// are `$3FF` because `min` clamps; `223721.56` — the deliberately absurd entry
/// the corpus comments on — comes out as 8.
#[test]
fn psg_frequencies_match_asl_including_the_min_clamp() {
    let got = bytes(&format!(
        "{S1_PSG_PRELUDE}\
\tirp op, 130.98, 138.78, 223721.56, 6991.28, 4142.98
\tdc.w MakePSGFrequency(op)
\tendm
"
    ));
    assert_eq!(
        got,
        vec![0x03, 0xff, 0x03, 0xff, 0x00, 0x08, 0x00, 0xfe, 0x01, 0xac],
        "PSGFrequencies must match asl's listing bytes, clamp included"
    );
}

/// The type rule that decides bytes: `/` between two INTEGERS is truncating
/// integer division even inside `INT(...)`, so the float side never sees a 3.5.
///
/// asl listing, `.f1probe/f2.asm`:
///
/// ```text
///        4/       0 : FFFF FFFD           	dc.l INT(-7/2)
///        5/       4 : 0000 0000           	dc.l INT(1/3*3)
///        6/       8 : 0000 0002           	dc.l INT(3/2*2)
/// ```
///
/// An evaluator that promotes to f64 on sight answers `floor(-3.5)` = -4,
/// `floor(0.333…*3)` = 0 (coincidentally right) and `floor(1.5*2)` = 3 — two
/// wrong longs out of a program that assembles clean, which is why this is
/// asserted rather than left to the ROM gate.
#[test]
fn integer_division_stays_integer_inside_int() {
    assert_eq!(
        bytes("\tdc.l INT(-7/2),INT(1/3*3),INT(3/2*2)\n"),
        vec![
            0xff, 0xff, 0xff, 0xfd, //
            0x00, 0x00, 0x00, 0x00, //
            0x00, 0x00, 0x00, 0x02,
        ],
        "int/int must stay integer division inside INT()"
    );
}

/// `INT()` is FLOOR, not truncation toward zero, and an integer argument passes
/// through unchanged.
///
/// asl listing, `.f1probe/f1.asm`:
///
/// ```text
///        4/       0 : 0000 0003           	dc.l INT(3.7)
///        6/       8 : FFFF FFFC           	dc.l INT(-3.7)
///        7/       C : FFFF FFFC           	dc.l INT(-3.2)
///        9/      14 : FFFF FFFD           	dc.l INT(-3.0)
///       11/      1C : 0000 0007           	dc.l INT(7)
/// ```
#[test]
fn int_floors_and_passes_integers_through() {
    assert_eq!(
        bytes("\tdc.l INT(3.7),INT(-3.7),INT(-3.2),INT(-3.0),INT(7)\n"),
        vec![
            0x00, 0x00, 0x00, 0x03, //
            0xff, 0xff, 0xff, 0xfc, //
            0xff, 0xff, 0xff, 0xfc, //
            0xff, 0xff, 0xff, 0xfd, //
            0x00, 0x00, 0x00, 0x07,
        ],
        "INT() must floor, and must pass an integer argument through"
    );
}

/// The corpus's rounding idiom, `roundFloatToInteger(x)` = `INT(x+0.5)`, at the
/// half-way points where floor and round-to-nearest disagree on the negatives.
///
/// asl listing, `.f1probe/f1.asm`:
///
/// ```text
///       13/      20 : 0000 0003           	dc.l INT(2.5+0.5)
///       14/      24 : FFFF FFFE           	dc.l INT(-2.5+0.5)
///       15/      28 : FFFF FFFD           	dc.l INT(-3.5+0.5)
/// ```
#[test]
fn round_idiom_matches_asl_at_the_halfway_points() {
    assert_eq!(
        bytes("\tdc.l INT(2.5+0.5),INT(-2.5+0.5),INT(-3.5+0.5)\n"),
        vec![
            0x00, 0x00, 0x00, 0x03, //
            0xff, 0xff, 0xff, 0xfe, //
            0xff, 0xff, 0xff, 0xfd,
        ],
    );
}

/// asl matches builtin FUNCTION names case-insensitively even under `-U`, which
/// makes user SYMBOLS case-sensitive. Both spellings assemble to 3
/// (`.f1probe/f1.asm` lines 4 and 10), and S1's `MacroSetup.asm:218` is written
/// in capitals — `roundFloatToInteger function float,INT(float+0.5)` — so the
/// lower-case-only match this replaces could not read the corpus at all.
#[test]
fn int_is_case_insensitive() {
    assert_eq!(
        bytes("\tdc.l INT(3.7),int(3.7),Int(3.7),iNt(3.7)\n"),
        vec![0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 3],
        "asl matches builtin function names case-insensitively"
    );
}

/// A float reaching an integer context is a REFUSAL, not a silent truncation.
///
/// asl, probe `.f1probe/f1.asm(17-19)` and `f3.asm(5)`:
///
/// ```text
/// > > > f1.asm(17):7: error #1133: expected integer or string, but got floating point number
/// > > > 3.7
/// >>>  dc.l 3.7
/// ```
///
/// The float SYMBOL case is the one that matters most: without the check it
/// parses as a bare `Expr::Sym`, folds to Poison and defers to the LINKER,
/// which reports a symbol that has a perfectly good value as an undefined one.
#[test]
fn a_float_in_an_integer_context_is_refused() {
    for body in [
        "\tdc.l 3.7\n",
        "\tdc.w 3.7\n",
        "\tdc.b 3.7\n",
        "fx = 3.7\n\tdc.l fx\n",
        "fx := 3.7\n\tdc.w fx*2\n",
    ] {
        let msgs = refused(body);
        assert!(
            msgs.iter().any(|m| m.contains("floating point")),
            "expected a floating-point diagnostic for `{}`, got {msgs:?}",
            body.trim()
        );
    }
}

/// But a float UNDER A COMPARISON is fine, because the comparison's result is
/// an integer. asl listing, `.f1probe/f2.asm`:
///
/// ```text
///       16/      20 : 0000 0001           	dc.l 3.5<4
/// ```
///
/// This is why the check is "the operand must REDUCE to an integer" rather than
/// "no float token may appear": the blunt reading refuses a line asl accepts.
#[test]
fn a_float_comparison_yields_an_integer_and_is_accepted() {
    assert_eq!(
        bytes("\tdc.l 3.5<4,INT(1.5<2),INT(2.5>2)\n"),
        vec![0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1],
    );
}

/// A symbol may HOLD a float. asl's listing prints the binding and its symbol
/// table carries the value (`.f1probe/f2.asm`):
///
/// ```text
///        8/       C : =3.7                 fx = 3.7
///       11/      14 : =2.5                 fy equ 2.5
///        fx :                           3.7 - |  fy :                           2.5 - |
///        9/       C : 0000 0003           	dc.l INT(fx)
///       10/      10 : 0000 0004           	dc.l INT(fx+1)
///       12/      14 : 0000 0005           	dc.l INT(fy*2)
/// ```
#[test]
fn a_symbol_may_hold_a_float() {
    assert_eq!(
        bytes("fx = 3.7\nfy equ 2.5\n\tdc.l INT(fx),INT(fx+1),INT(fy*2)\n"),
        vec![0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5],
    );
}

/// The reassignable form, which is the shape `s2.sounddriver.asm(3901)` uses:
/// `sample_rate_scale := 1.0`, overwritten by an optional macro parameter, then
/// read through `int(label.sample_rate*sample_rate_scale)`.
///
/// asl, `.f1probe/f4.asm` (`sc := 1.0` then `sc := 1.30`, each followed by
/// `dc.b INT(100*sc)`): trailing listing bytes `64 82`.
///
/// The reassignment is the load-bearing half — a float symbol whose earlier
/// value outlived its own assignment would silently scale every later sample.
#[test]
fn a_float_set_symbol_is_reassignable() {
    assert_eq!(
        bytes("sc := 1.0\n\tdc.b INT(100*sc)\nsc := 1.30\n\tdc.b INT(100*sc)\n"),
        vec![0x64, 0x82],
    );
}

/// A float symbol REASSIGNED TO AN INTEGER reads back as that integer — the
/// direction where a stale float entry would otherwise win the lookup.
///
/// asl listing, `.f1probe/f5.asm`:
///
/// ```text
///        6/      10 : =1.5                 sc := 1.5
///        7/      10 : =$7                  sc := 7
///        8/      10 : 07                  	dc.b sc
/// ```
#[test]
fn reassigning_a_float_symbol_to_an_integer_clears_the_float() {
    assert_eq!(bytes("sc := 1.5\nsc := 7\n\tdc.b sc\n"), vec![0x07]);
}

/// `int(...)` must work at EVERY `dc` width, not only `dc.b`. That asymmetry —
/// the builtin layer wired into `dc.b` alone — is the whole of the 166.
///
/// asl listing, `.f1probe/f7.asm` (the `00` at offset 1 is the 68000's
/// automatic even-padding before the word, `padding` being on by default):
///
/// ```text
///        2/       0 : 03                  	dc.b INT(3.7)
///        3/       1 : 00                  <padding>
///        3/       2 : 0003                	dc.w INT(3.7)
///        4/       4 : 0000 0003           	dc.l INT(3.7)
/// asl bytes: 03 00 00 03 00 00 00 03
/// ```
#[test]
fn int_works_at_every_dc_width() {
    assert_eq!(
        bytes("\tdc.b INT(3.7)\n\tdc.w INT(3.7)\n\tdc.l INT(3.7)\n"),
        vec![0x03, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03],
    );
}

/// The Z80 `db`/`dw` spellings share the widths' one expansion helper.
/// asl (`cpu z80`, `db INT(3.7)` / `dw INT(600.7)`) emits `03` then the
/// little-endian `58 02`.
#[test]
fn int_works_in_z80_db_and_dw() {
    let src = "\tcpu z80\n\tphase 0\n\tdb INT(3.7)\n\tdw INT(600.7)\n";
    let module = assemble(src, &Options::default()).expect("assemble");
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    assert_eq!(sigil_link::flatten(&linked, 0x00), vec![0x03, 0x58, 0x02]);
}
