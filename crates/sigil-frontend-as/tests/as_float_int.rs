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
//!
//! WHY SEVERAL ITEMS BELOW CARRY `#[allow(clippy::tabs_in_doc_comments)]`. The
//! `text` blocks are asl listings pasted verbatim, and asl separates its columns
//! with TABS. Those tabs are the evidence: what these comments assert is what the
//! reference assembler PRINTED, so respacing them into four spaces would quietly
//! restate the claim about output asl never produced. The waiver is per-item and
//! deliberately NOT a file-scoped `#![allow]`: a new test here that grows a tab by
//! accident should still trip the lint and be looked at, which a file-wide allow
//! would silently absorb.

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
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only — see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
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
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only — see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
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
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only — see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
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
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only — see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
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
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only — see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
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
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only — see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
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
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only — see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn int_works_at_every_dc_width() {
    assert_eq!(
        bytes("\tdc.b INT(3.7)\n\tdc.w INT(3.7)\n\tdc.l INT(3.7)\n"),
        vec![0x03, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x03],
    );
}

/// The wiring, pinned with an `int()` whose argument holds NO float token.
///
/// There are two routes by which a float can reach an integer here, and only
/// one of them is the wiring under test. `expand_int_builtin` substitutes each
/// `int(...)` span with a `Tok::Int` and leaves the REST of the expression to
/// the ordinary integer folder — which is what keeps labels, forward
/// references and link deferral working around it. `collapse_float_operand`
/// runs after, and only if a float LEAF survived that; it evaluates the whole
/// operand with the typed evaluator, which is how `dc.l 3.5<4` resolves.
///
/// The second route can answer an `int(3.7)` on its own, so a test whose
/// operand contains a float literal does NOT pin the first one. These operands
/// contain no float token at all, so nothing but the wiring can satisfy them.
///
/// asl listing, `.f1probe/f8.asm`:
///
/// ```text
///        2/       0 : 07                  	dc.b INT(7)
///        3/       1 : 00                  <padding>
///        3/       2 : 0258                	dc.w INT(600)
///        4/       4 : FFFF FFFD           	dc.l INT(-7/2)
///        5/       8 : 0101                	dc.w INT(1.5)+$100
/// ```
///
/// The last line is the layering itself: `int(...)` collapses to a token and
/// the `+$100` is folded by the integer path, exactly as
/// `dc.w MakeFMFrequency(op)+octave*$800` needs.
#[test]
// The tabs in the listing above are asl's own field separators, so they are part of
// the evidence rather than formatting; respacing them would restate the claim about
// output asl never produced. Waived here only — see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn int_with_an_integer_argument_pins_the_wiring_at_every_width() {
    assert_eq!(
        bytes("\tdc.b INT(7)\n\tdc.w INT(600)\n\tdc.l INT(-7/2)\n\tdc.w INT(1.5)+$100\n"),
        vec![0x07, 0x00, 0x02, 0x58, 0xff, 0xff, 0xff, 0xfd, 0x01, 0x01],
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

// ── The float FUNCTION surface (`log`, `ln`, `sqrt`, the trig family) ────────
//
// Oracle for everything below: the same asl build as the rest of this file,
// probe `docs/superpowers/notes/2026-09-06-as-float-freq-table-probes/values2.asm`,
// run through `asl_run` with `-xx -n -q -A -L -U -i .`. That run reports
// `ASL_EXIT=0` and `ASL_DIAG=complete`, which is why its byte column may be
// quoted at all: a run carrying any error leaves forward references at their
// pass-1 placeholders and prints them looking complete.
//
// `values2.asm` is deliberately written without `1e6`-style exponent literals,
// which sigil's lexer does not implement, so the SAME source text goes to both
// assemblers and the comparison is not mediated by a rewrite.

/// `s2.asm(87675-87682)`'s `hud_counter`, verbatim, plus the `moveq` that reads
/// the counter back (`s2.asm(87595)`, `(87746)`, and four more).
///
/// This is the whole parcel in one fixture. `.loop_counter` is "total digits
/// minus one" and is consumed as a `moveq` immediate, so a wrong `log` is a
/// wrong shipped byte and not merely a diagnostic.
const S2_HUD_COUNTER: &str = "\
hud_counter macro {INTLABEL},number
__LABEL__ label *
.loop_counter = int(log(number)) ; Total digits minus one.
\tdc.l number
    endm
Hud_100000:\thud_counter 100000
Hud_10000:\thud_counter 10000
Hud_1000:\thud_counter 1000
Hud_100:\thud_counter 100
Hud_10:\thud_counter 10
Hud_1:\thud_counter 1
\tmoveq\t#Hud_100000.loop_counter,d6
\tmoveq\t#Hud_10000.loop_counter,d6
\tmoveq\t#Hud_1000.loop_counter,d6
\tmoveq\t#Hud_100.loop_counter,d6
\tmoveq\t#Hud_10.loop_counter,d6
\tmoveq\t#Hud_1.loop_counter,d6
";

/// asl listing, `values2.asm` lines 20-32 (macro expansions shown):
///
/// ```text
///       22/       8 : =$3                  .loop_counter = int(log(1000))
///       27/      18 : 7C05                	moveq	#Hud_100000.loop_counter,d6
///       28/      1A : 7C04                	moveq	#Hud_10000.loop_counter,d6
///       29/      1C : 7C03                	moveq	#Hud_1000.loop_counter,d6
///       30/      1E : 7C02                	moveq	#Hud_100.loop_counter,d6
///       31/      20 : 7C01                	moveq	#Hud_10.loop_counter,d6
///       32/      22 : 7C00                	moveq	#Hud_1.loop_counter,d6
/// ```
///
/// The `moveq` half is the reason `log` must be spelled as a dedicated base-10
/// logarithm rather than `x.ln() / 10f64.ln()`. In binary64 the latter gives
/// 2.9999999999999996 for 1000, one ULP short of 3, and `int()` FLOORS — so
/// that spelling emits `7C02` for `Hud_1000` and the HUD counts a digit short.
/// The assertion below therefore discriminates the two spellings of the right
/// base, not merely the right base from the wrong one.
#[test]
// asl separates its listing columns with tabs; they are the evidence, not
// formatting. Waived here only — see the file's module doc.
#[allow(clippy::tabs_in_doc_comments)]
fn hud_counter_loop_counters_match_asl() {
    let got = bytes(S2_HUD_COUNTER);
    // The six `dc.l`s first, then the six `moveq`s. Spelled out in full rather
    // than sliced, so a fixture that silently stopped emitting the `moveq`
    // block — the way this assertion could go green over an empty population —
    // fails on length instead of passing on a prefix.
    assert_eq!(
        got,
        vec![
            0x00, 0x01, 0x86, 0xa0, // dc.l 100000
            0x00, 0x00, 0x27, 0x10, // dc.l 10000
            0x00, 0x00, 0x03, 0xe8, // dc.l 1000
            0x00, 0x00, 0x00, 0x64, // dc.l 100
            0x00, 0x00, 0x00, 0x0a, // dc.l 10
            0x00, 0x00, 0x00, 0x01, // dc.l 1
            0x7c, 0x05, // moveq #Hud_100000.loop_counter,d6
            0x7c, 0x04, // moveq #Hud_10000.loop_counter,d6
            0x7c, 0x03, // moveq #Hud_1000.loop_counter,d6   <- the ULP row
            0x7c, 0x02, // moveq #Hud_100.loop_counter,d6
            0x7c, 0x01, // moveq #Hud_10.loop_counter,d6
            0x7c, 0x00, // moveq #Hud_1.loop_counter,d6
        ],
    );
}

/// The naive spelling really does differ, stated as arithmetic rather than as a
/// claim about an implementation nobody wrote.
///
/// Without this, `hud_counter_loop_counters_match_asl` looks like it only pins
/// "base 10", and a later reader could swap `log10` for `ln(x)/ln(10)` believing
/// the tests cover it. 1000 is the one argument among the six where they differ.
#[test]
fn ln_over_ln10_is_a_ulp_short_at_1000() {
    let naive = 1000f64.ln() / 10f64.ln();
    assert_ne!(naive, 3.0, "the naive spelling would be exact and the risk imaginary");
    assert_eq!(naive.floor(), 2.0, "and `int()` floors it to the wrong digit count");
    assert_eq!(1000f64.log10(), 3.0, "while the dedicated log10 is exact");
}

/// `log` is base 10, and this build has the natural one under its own name.
///
/// asl listing, `values2.asm`:
///
/// ```text
///       35/      24 : 0000 0002           	dc.l	INT(LOG(100))
///       36/      28 : 0000 0004           	dc.l	INT(LN(100))
///       37/      2C : 0000 0003           	dc.l	INT(LOG(1000))
/// ```
///
/// `LOG(100)` = 2 is the base discriminator: the natural log of 100 is 4.605,
/// which floors to 4 — the value the very next row shows `LN` actually
/// produces. A probe on `LOG(10)` would have been useless here, since it
/// answers 1 under several wrong readings.
#[test]
#[allow(clippy::tabs_in_doc_comments)]
fn log_is_base_ten_and_ln_is_natural() {
    assert_eq!(bytes("\tdc.l INT(LOG(100))\n"), vec![0x00, 0x00, 0x00, 0x02]);
    assert_eq!(bytes("\tdc.l INT(LN(100))\n"), vec![0x00, 0x00, 0x00, 0x04]);
    assert_eq!(bytes("\tdc.l INT(LOG(1000))\n"), vec![0x00, 0x00, 0x00, 0x03]);
}

/// Builtin function names are matched case-insensitively even under `-U`, which
/// makes user SYMBOLS case-sensitive. The corpus writes `log` in lower case
/// (`s2.asm(87677)`); every probe in this parcel writes `LOG`. asl assembles
/// `INT(log(1000))`, `INT(Log(1000))` and `INT(lOg(1000))` all to `0000 0003`
/// (`clean2.asm` lines 12-14, `ASL_EXIT=0`).
#[test]
fn builtin_names_are_case_insensitive() {
    for spelling in ["log", "LOG", "Log", "lOg"] {
        assert_eq!(
            bytes(&format!("\tdc.l INT({spelling}(1000))\n")),
            vec![0x00, 0x00, 0x00, 0x03],
            "{spelling}"
        );
    }
}

/// `int()` FLOORS toward negative infinity; it does not truncate toward zero.
///
/// No positive argument can tell those apart, which is why every row here is
/// negative. asl listing, `values2.asm`:
///
/// ```text
///       40/      30 : FFFF FFFC           	dc.l	INT(-3.2)
///       41/      34 : FFFF FFFF           	dc.l	INT(LOG(0.5))
///       42/      38 : FFFB 681A           	dc.l	INT(LOG(0.5)*1000000)
/// ```
///
/// -4, -1 and -301030. Truncation toward zero would give -3, 0 and -301029, so
/// all three rows discriminate — and the third does it six digits into the new
/// function rather than at a value where the two happen to meet.
#[test]
#[allow(clippy::tabs_in_doc_comments)]
fn int_floors_a_negative_including_through_log() {
    assert_eq!(bytes("\tdc.l INT(-3.2)\n"), vec![0xff, 0xff, 0xff, 0xfc]);
    assert_eq!(bytes("\tdc.l INT(LOG(0.5))\n"), vec![0xff, 0xff, 0xff, 0xff]);
    assert_eq!(
        bytes("\tdc.l INT(LOG(0.5)*1000000)\n"),
        vec![0xff, 0xfb, 0x68, 0x1a],
    );
}

/// The rest of the surface, one row per builtin, each at an argument where a
/// plausible wrong implementation answers visibly differently.
///
/// asl listing, `values2.asm` lines 45-60, in this order:
///
/// ```text
///       45/      3C : 0070 BF80           	dc.l	INT(EXP(2)*1000000)
///       46/      40 : 0015 9445           	dc.l	INT(SQRT(2)*1000000)
///       47/      44 : 000C D6FE           	dc.l	INT(SIN(1)*1000000)
///       48/      48 : 0008 3E8E           	dc.l	INT(COS(1)*1000000)
///       49/      4C : 0017 C39F           	dc.l	INT(TAN(1)*1000000)
///       50/      50 : 000B FBF6           	dc.l	INT(ATAN(1)*1000000)
///       51/      54 : 0017 F7EC           	dc.l	INT(ASIN(1)*1000000)
///       52/      58 : 0017 F7EC           	dc.l	INT(ACOS(0)*1000000)
///       53/      5C : 0011 EEA1           	dc.l	INT(SINH(1)*1000000)
///       54/      60 : 0017 8BA8           	dc.l	INT(COSH(1)*1000000)
///       55/      64 : 000B 9EFA           	dc.l	INT(TANH(1)*1000000)
///       56/      68 : 000D 72DD           	dc.l	INT(ASINH(1)*1000000)
///       57/      6C : 0014 185D           	dc.l	INT(ACOSH(2)*1000000)
///       58/      70 : 0008 61BA           	dc.l	INT(ATANH(0.5)*1000000)
///       59/      74 : 0031 9750           	dc.l	INT(ABS(-3.25)*1000000)
///       60/      78 : 0000 0003           	dc.l	INT(ABS(-3))
/// ```
///
/// Why each argument and not a rounder one:
/// * `EXP(2)` = 7389056, where `2^x` would give 4000000. `EXP(1)` would not
///   have separated them — both floor to 2.
/// * `SIN(1)`, `COS(1)`, `TAN(1)`, `ATAN(1)`, `ASIN(1)`, `ACOS(0)` are all in
///   RADIANS: 841470 / 540302 / 1557407 / 785398 / 1570796 / 1570796. Reading
///   the argument as degrees gives 17452 / 999847 / 17455 / 45000000 /
///   90000000 / 90000000 — a different digit count, not a rounding difference.
/// * `ABS(-3)` is the type row: it stays an INTEGER. Every other function here
///   returns a float even when the value is integral, which asl shows from the
///   failing side — `dc.l SQRT(16)` is `error #1133` (`types.asm(10)`) while
///   `dc.l ABS(-3)` assembles to `0000 0003` (`clean2.asm(8)`). The row below
///   asks it through `INT(...)`, which is the only integer context sigil routes
///   to this evaluator; a BARE `dc.l ABS(-3)` is still refused here, and is
///   recorded as a gap rather than claimed.
#[test]
#[allow(clippy::tabs_in_doc_comments)]
fn float_builtin_surface_matches_asl() {
    let rows: &[(&str, u32)] = &[
        ("INT(EXP(2)*1000000)", 0x0070_BF80),
        ("INT(SQRT(2)*1000000)", 0x0015_9445),
        ("INT(SIN(1)*1000000)", 0x000C_D6FE),
        ("INT(COS(1)*1000000)", 0x0008_3E8E),
        ("INT(TAN(1)*1000000)", 0x0017_C39F),
        ("INT(ATAN(1)*1000000)", 0x000B_FBF6),
        ("INT(ASIN(1)*1000000)", 0x0017_F7EC),
        ("INT(ACOS(0)*1000000)", 0x0017_F7EC),
        ("INT(SINH(1)*1000000)", 0x0011_EEA1),
        ("INT(COSH(1)*1000000)", 0x0017_8BA8),
        ("INT(TANH(1)*1000000)", 0x000B_9EFA),
        ("INT(ASINH(1)*1000000)", 0x000D_72DD),
        ("INT(ACOSH(2)*1000000)", 0x0014_185D),
        ("INT(ATANH(0.5)*1000000)", 0x0008_61BA),
        ("INT(ABS(-3.25)*1000000)", 0x0031_9750),
        ("INT(ABS(-3))", 0x0000_0003),
    ];
    // A loop over a table is exactly the shape that passes by examining
    // nothing, so the table's own size is pinned against the count of `dc.l`
    // rows read out of the listing above.
    assert_eq!(rows.len(), 16, "asl listing values2.asm lines 45-60");
    for (expr, want) in rows {
        assert_eq!(
            bytes(&format!("\tdc.l {expr}\n")),
            want.to_be_bytes().to_vec(),
            "{expr}"
        );
    }
}

/// A float RESULT in an integer slot is refused even when the value is a whole
/// number, because the builtins return a float TYPE rather than a float value.
///
/// asl, `types.asm` (a deliberate-failure probe — read for its diagnostics, not
/// its byte column): `dc.l LOG(100)` and `dc.l SQRT(16)` each draw `error
/// #1133: expected integer or string, but got floating point number`, and
/// `dc.l LOG(100)&1` draws `#1134: expected integer, but got floating point
/// number`.
///
/// WHAT THIS TEST DOES AND DOES NOT PIN, stated because it passes on the
/// pre-`log` baseline too and so proves nothing about the new table on its own.
/// It pins the DIRECTION only — that these lines are refused rather than
/// emitted. sigil's wording is its generic `bad long expression`, not asl's
/// `#1133`, because the operand path reaches the typed evaluator only when a
/// float TOKEN is present and `LOG(100)` contains none. That message gap is a
/// recorded gap, not a claim; what matters for bytes is that no value is
/// invented, and this asserts exactly that.
#[test]
fn a_float_builtin_result_is_refused_in_an_integer_slot() {
    let bodies = ["\tdc.l LOG(100)\n", "\tdc.l SQRT(16)\n", "\tdc.l LOG(100)&1\n"];
    assert_eq!(bodies.len(), 3, "types.asm lines 7, 10, 11");
    for body in bodies {
        assert!(!refused(body).is_empty(), "{body}");
    }
}

/// An argument outside a function's definition range is REFUSED, not carried as
/// a NaN or an infinity into the byte column.
///
/// asl, `domain.asm`: `LOG(0)`, `LOG(-1)`, `SQRT(-1)` and `ASIN(2)` each draw
/// `error #1870: function argument out of definition range`; `ATANH(2)` and
/// `ACOSH(0)` draw `#1880: floating point overflow`. All six are exactly the
/// arguments where the corresponding `f64` method returns a non-finite value.
///
/// The last row is the same class from the other end: asl answers `error #1320:
/// range overflow` at the INT itself — `bigint.asm(8)` writes
/// `dc.l INT(1e30)-INT(1e30)`, whose value is 0 and in range for a `dc.l`, and
/// asl reports the overflow twice on that one line. An unguarded
/// `f.floor() as i64` in Rust saturates to `i64::MAX` instead and emits a
/// plausible number. (`1e30` is spelled out as a product because sigil's lexer
/// has no exponent literals.)
#[test]
fn an_out_of_domain_or_out_of_range_argument_is_refused() {
    let bodies = [
        "\tdc.l INT(LOG(0))\n",
        "\tdc.l INT(LOG(-1))\n",
        "\tdc.l INT(SQRT(-1))\n",
        "\tdc.l INT(ASIN(2))\n",
        "\tdc.l INT(ATANH(2))\n",
        "\tdc.l INT(ACOSH(0))\n",
        "\tdc.l INT(1000000000000000.0*1000000000000000.0)\n",
    ];
    assert_eq!(bodies.len(), 7, "six domain rows plus the range row");
    for body in bodies {
        assert!(!refused(body).is_empty(), "{body}");
    }
}

/// An identifier that is NOT a builtin still reaches the ordinary symbol
/// lookup. Guarding this direction is what stops the new name table from
/// swallowing a user symbol that happens to be followed by a parenthesis.
///
/// asl calls an unknown one out by name — `error #1860: unknown function
/// BOGUSFN` (`names.asm(15)`), uppercased, which is the same lookup seen from
/// the failing side.
#[test]
fn an_unknown_function_name_is_not_treated_as_a_builtin() {
    assert!(!refused("\tdc.l INT(BOGUSFN(1))\n").is_empty());
    // And a symbol whose name merely STARTS with a builtin keeps its value.
    assert_eq!(bytes("logo = 7\n\tdc.l INT(logo)\n"), vec![0, 0, 0, 7]);
}
