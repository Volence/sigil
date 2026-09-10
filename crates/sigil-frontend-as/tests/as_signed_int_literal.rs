//! asl's `+`-SIGNED INTEGER LITERAL: `+1`, `+$FF`, `+%1010`, and the exact edge
//! of where a leading `+` stops being one.
//!
//! ## What was silently wrong
//!
//! `parse_atom` carried an arm for unary MINUS and none for `+`, so every
//! expression containing one failed to PARSE. In a `dc.b` that is loud; in a
//! `set` it was completely silent, because `directive_set`'s last resort is
//! `defer_unresolved_assign`, whose first statement returns without a word when
//! the right-hand side does not parse — leaving the symbol's PREVIOUS value
//! standing:
//!
//! ```text
//!     set .val, $21
//!     dc.b .val
//!     set .val, .val+(+1)
//!     dc.b .val
//!     set .val, .val+(+1)
//!     dc.b .val
//!
//!     asl 21 22 23        sigil 21 21 21, exit 0, no diagnostic
//! ```
//!
//! Sonic 1 reaches it through `Macros.asm(346)`'s `range` macro, whose body ends
//! `set .val, .val+(step)`: a call with a `+`-signed step substitutes to exactly
//! that shape. Four call sites, 80 emitted bytes, 75 of them wrong (each range's
//! FIRST byte is right — it is the increment that never happened), exit 0 and
//! not one diagnostic.
//!
//! ## The reference
//!
//! Every expectation below is a measured `dc.l` of the expression under the
//! reference `asl`, md5 `61e672562465725a8c102288a7da9098`, exit status checked;
//! a refusal expectation is that asl's own exit 2 with `error #1110: wrong
//! number of operands`. asl has NO unary-plus operator: it tries a
//! (sub)expression as a literal first and otherwise splits it at the rightmost
//! operator of the loosest tier present, where a leading `+` is a binary add
//! with an empty left operand. Unary MINUS is a real operator there, which is
//! why `-2*3` folds and `+2*3` refuses.

use sigil_frontend_as::{assemble, Options};

/// The image of a source that must assemble.
fn image(src: &str) -> Vec<u8> {
    match assemble(src, &Options::default()) {
        Ok(m) => m.sections.first().map(|s| s.image_bytes()).unwrap_or_default(),
        Err(ds) => panic!(
            "expected assembly to succeed; refused with {:?}",
            ds.into_iter().map(|d| d.message).collect::<Vec<_>>()
        ),
    }
}

/// A `dc.l` of one expression, big-endian, with a label and an `equ` in scope so
/// the non-literal shapes have something real to name.
fn dcl(expr: &str) -> String {
    format!("\tcpu 68000\n\tpadding off\n\torg 0\nBase:\nSZ:\tequ 8\n\tdc.l {expr}\n\tend\n")
}

/// Fold one expression to its 32-bit value, failing if it is refused.
fn value(expr: &str) -> u32 {
    let b = image(&dcl(expr));
    assert_eq!(b.len(), 4, "`{expr}` emitted {} byte(s), not a dc.l", b.len());
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

/// Whether one expression is refused. The refusal must be a DIAGNOSTIC, never a
/// silent zero-length image: this whole defect class is silence.
fn is_refused(expr: &str) -> bool {
    match assemble(&dcl(expr), &Options::default()) {
        Ok(m) => {
            let n = m.sections.first().map(|s| s.image_bytes().len()).unwrap_or(0);
            assert!(n > 0, "`{expr}` was accepted but emitted nothing: a silent refusal");
            false
        }
        Err(ds) => {
            assert!(!ds.is_empty(), "`{expr}` was refused with no diagnostic");
            true
        }
    }
}

#[test]
fn a_leading_plus_on_a_number_is_that_number() {
    for (expr, want) in [
        ("+1", 1u32),
        ("+0", 0),
        ("+2", 2),
        ("+$FF", 0xFF),
        ("+%1010", 0b1010),
        ("+123456", 123_456),
    ] {
        assert_eq!(value(expr), want, "`{expr}`");
    }
}

#[test]
fn a_parenthesised_group_is_a_head_position() {
    for (expr, want) in [
        ("(+1)", 1u32),
        ("(+$10)", 0x10),
        ("(((+1)))", 1),
        ("$21+(+1)", 0x22),
        ("1+(+1)", 2),
        ("1-(+1)", 0),
        ("(+1)+(+1)", 2),
        ("(+1)*3", 3),
        ("(+1)&2", 0),
    ] {
        assert_eq!(value(expr), want, "`{expr}`");
    }
}

#[test]
fn the_add_tier_and_looser_may_follow_the_literal() {
    // asl splits at the LOOSEST tier present, so an operator at the `+` tier or
    // below leaves the signed literal intact as the left leaf.
    for (expr, want) in [
        ("+1+2", 3u32),
        ("+1-2", 0xFFFF_FFFF),
        ("+8-SZ", 0),
        ("+1+2*3", 7),
        ("+1+2*3&4", 1),
        ("+1&&2", 1),
        ("+1||2", 1),
        ("+1=2", 0),
        ("+1<2", 1),
        ("+1&&2*3", 1),
    ] {
        assert_eq!(value(expr), want, "`{expr}`");
    }
}

#[test]
fn an_operator_tighter_than_add_strands_the_sign() {
    // asl's loosest-tier split then lands on the leading `+` itself, which is a
    // binary add with no left operand: `error #1110`, exit 2.
    for expr in [
        "+1*3", "+1/2", "+1#2", "+1&2", "+1|2", "+1!2", "+1<<2", "+1*2+3", "+1*2&&3", "+1<<2+3",
        "(+1*3)", "+2*3", "2*+3", "2*3++1",
    ] {
        assert!(is_refused(expr), "`{expr}` folded; asl refuses it with #1110");
    }
}

#[test]
fn the_sign_must_be_adjacent_and_at_a_head() {
    // `+ 1`: asl does not strip the blank before trying the literal.
    // `1+ +2`: the bare `+2` never gets a string of its own to be scanned as.
    // The rest are not numbers at all.
    for expr in ["+ 1", "(+ 1)", "1+ +2", "+Base", "+SZ", "+(1)", "+(1+2)", "+~0", "~+0"] {
        assert!(is_refused(expr), "`{expr}` folded; asl refuses it with #1110");
    }
}

#[test]
fn unary_minus_is_untouched() {
    // asl DOES have this operator, and the `+` arm must not have disturbed it.
    for (expr, want) in [
        ("-1", 0xFFFF_FFFFu32),
        ("-SZ", 0xFFFF_FFF8),
        ("-(SZ*2)", 0xFFFF_FFF0),
        ("-2*3", 0xFFFF_FFFA),
        ("-$FF", 0xFFFF_FF01),
        ("-~SZ", 9),
        ("abs(+3)", 3),
    ] {
        assert_eq!(value(expr), want, "`{expr}`");
    }
}

/// The residual documented on `expr.rs::signed_int_literal`, pinned so it cannot
/// move without a test saying so.
///
/// A `Tok::Int` cannot say whether it came from numeric-literal syntax, so a
/// character constant (packed by the lexer) and a builtin the evaluator folds to
/// one integer before the parse both look like literals here. asl sees the
/// unfolded TEXT and raises `#1110` for all of them. ACCEPT-MORE in every case —
/// sigil folds where asl refuses, never to a different value — so it cannot put
/// a wrong byte in an image, and no source carrying one of these shapes builds
/// under asl at all.
#[test]
fn known_residual_a_packed_or_folded_int_also_takes_the_sign() {
    for (expr, folds_to) in [
        ("+'A'", 0x41u32),
        ("+'AB'", 0x4142),
        ("+defined(SZ)", 1),
        ("+abs(-3)", 3),
        ("+strlen(\"ab\")", 2),
    ] {
        assert_eq!(
            value(expr),
            folds_to,
            "`{expr}`: asl raises #1110 here; if sigil now refuses it too, the residual has been \
             CLOSED and this test should become an `is_refused` assertion"
        );
    }
}

/// Sonic 1's `Macros.asm(346)`, verbatim. The last line of the loop body is the
/// whole reach: a `+`-signed `step` substitutes into `set .val, .val+(step)`.
const S1_RANGE: &str = "\
range: macro first,last,step,repeat\n\
\tset .rep, 1\n\
\tif \"repeat\"<>\"\"\n\
\t\tset .rep, repeat\n\
\tendif\n\
\n\
\tset .val, first\n\
\trept 1+(abs(first-last)/abs(step))\n\
\t\trept .rep\n\
\t\t\tdc.b .val\n\
\t\tendr\n\
\t\tset .val, .val+(step)\n\
\tendr\n\
\tendm\n";

fn range_call(call: &str) -> Vec<u8> {
    image(&format!("\tcpu 68000\n\tpadding off\n\torg 0\n{S1_RANGE}\t{call}\n\tend\n"))
}

#[test]
fn the_four_sonic_1_range_sites_match_asl() {
    // Each expectation is the asl image of the same call, reference build
    // md5 `61e672562465725a8c102288a7da9098`, exit 0. Pre-fix sigil emitted the
    // first byte repeated for the whole run, exit 0, no diagnostic: 14, 30, 28
    // and 3 wrong bytes respectively.

    // `_incObj/2F, 35 MZ Large Grassy Platforms and Burning Grass.asm:322`
    assert_eq!(range_call("range\t$21,$2F,+1"), (0x21u8..=0x2F).collect::<Vec<_>>());

    // the same file, line 336
    assert_eq!(range_call("range\t$21,$3F,+1"), (0x21u8..=0x3F).collect::<Vec<_>>());

    // `_incObj/1A, 53 Collapsing Ledges and Floors.asm:441` — each step twice
    let doubled: Vec<u8> = (0x21u8..=0x2F).flat_map(|b| [b, b]).collect();
    assert_eq!(range_call("range\t$21,$2F,+1,2"), doubled);

    // `_incObj/5E SLZ Seesaw.asm:310`
    assert_eq!(range_call("range\t$26,$2C,+2"), vec![0x26, 0x28, 0x2A, 0x2C]);
}

#[test]
fn the_descending_sonic_1_range_sites_are_unchanged() {
    // The `-`-stepped calls that share those data blocks always worked; they are
    // here so the `+` arm cannot fix one direction by breaking the other.
    assert_eq!(range_call("range\t$2F,$21,-1"), (0x21u8..=0x2F).rev().collect::<Vec<_>>());
    assert_eq!(range_call("range\t$3F,$31,-1"), (0x31u8..=0x3F).rev().collect::<Vec<_>>());
    assert_eq!(range_call("range\t$2A,$24,-2"), vec![0x2A, 0x28, 0x26, 0x24]);
    assert_eq!(range_call("range\t$23,$03,-1"), (0x03u8..=0x23).rev().collect::<Vec<_>>());
}

/// The reported repro, whole. It is a `set` defect, not a `rept` one — the
/// nesting is a bystander — so the flat form is asserted beside it.
#[test]
fn the_reported_repro_and_its_flat_form() {
    // asl: `21 21 22 22 23 23 24 24`.
    let nested = "\tcpu 68000\n\tpadding off\n\torg 0\n\tset .rep, 2\n\tset .val, $21\n\
                  \trept 4\n\t\trept .rep\n\t\t\tdc.b .val\n\t\tendr\n\tset .val, .val+(+1)\n\
                  \tendr\n\tend\n";
    assert_eq!(image(nested), vec![0x21, 0x21, 0x22, 0x22, 0x23, 0x23, 0x24, 0x24]);

    // asl: `21 22 23`. No `rept` anywhere — this is the whole defect.
    let flat = "\tcpu 68000\n\tpadding off\n\torg 0\n\tset .val, $21\n\tdc.b .val\n\
                \tset .val, .val+(+1)\n\tdc.b .val\n\tset .val, .val+(+1)\n\tdc.b .val\n\tend\n";
    assert_eq!(image(flat), vec![0x21, 0x22, 0x23]);
}
