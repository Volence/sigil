//! AS's `[count]value` DUPLICATE-OPERAND syntax, and the `listing`/`page`
//! listing-file controls.
//!
//! Both are what Sonic 1's `MacroSetup.asm` opens with. `listing purecode` and
//! `page 0` sit at its lines 7 and 8, its `sound/z80.asm` writes `listing`
//! again on the Z80 side, and its `dcb` macro body is
//!
//! ```text
//!     dcb macro count,value
//!             dc.ATTRIBUTE    [count]value
//!         endm
//! ```
//!
//! which every `dcb.b N,V` in the disassembly expands into. Its `org0` macro
//! writes `dc.b [1024]0` and `dc.b [.diff]0` directly.
//!
//! ## The reference
//!
//! Every expectation below is derived from asl 1.42 Beta [Bld 212]
//! (`x86_64-unknown-linux`, md5 `61e672562465725a8c102288a7da9098`, the build
//! Sonic 1's own `build.lua` runs), invoked `-xx -n -q -A -L -U -i .` with the
//! exit status checked on every run and the listing discarded on a non-zero
//! one. The probe files are committed beside this crate at
//! `docs/superpowers/notes/2026-09-09-as-dup-operand-probes/`, one construct
//! per file so a refusal in one cannot poison another's byte column.
//!
//! ```text
//!   d1   dc.b [3]$FF / dc.w [2]$1234 / dc.l [2]$AABBCCDD
//!                                   -> FF FF FF / 1234 1234 / AABBCCDD x2,
//!                                      and `$` advances by the whole run
//!   d2   dc.b $01,[3]$FF,$02         -> 01 FF FF FF 02
//!   d3   [1+2] / [(2*2)] / [n] / [n-1] with `n equ 4`, all accepted
//!   d4   [ 3 ]$AA / [3] $BB / [ 2 ] $CC, whitespace immaterial
//!   d5   dc.b [0]$FF                 -> nothing, `$` does not move
//!   d6   a FORWARD-referenced count  -> error #1820, must be evaluatable
//!                                      in the first pass
//!   d7   dc.b [2]"ab"                -> 61 62 61 62
//!   d9   dc.b [2][3]$FF              -> error #1010 on the SYMBOL `[3]$FF`:
//!                                      only a LEADING bracket is a count
//!   d10  dc.b [-1]$FF                -> error #1920 code overflow
//!   d13  the `dcb` macro shape above, expanding to `dc.b [3]$FF` etc.
//!   d15  dc.b [3] with no value      -> 00 00 00  (see the divergence below)
//!   d17  [1024] / [1025] / [1596] / [$62A] all assemble, so `MacroSetup.asm`'s
//!        "AS can only generate 1 kb of code on a single line" does not bind
//!        this build
//!   l1   listing purecode / page 0 under `cpu 68000`, accepted, emits nothing
//!   l2   listing zqp_bogus           -> error #1520 only ON/OFF allowed
//!   l3   a bare `listing` / a bare `page` -> error #1110, wrong operand count
//!   l4   listing purecode / page 0 under `cpu z80`, accepted
//! ```
//!
//! ## Three deliberate divergences, pinned here so reversing one is deliberate
//!
//! `dc.b [3]` with NO value emits three zero bytes in asl (d15). This front end
//! refuses it by name instead: an empty operand meaning zero is a separate rule
//! that `dc` does not implement here, and inheriting it silently through the
//! bracket path would make a truncated line assemble.
//!
//! A count whose symbol is defined BELOW it is `expression must be evaluatable
//! in first pass` in asl (d6) and resolves here, because this front end runs to
//! convergence rather than in one pass. The bytes are the ones the source means.
//!
//! asl validates the `listing` ARGUMENT (l2) and this front end does not. asl's
//! own message for a rejected one says `only ON/OFF allowed` while it accepts
//! `purecode`, so the set it takes is wider than any wording available here, and
//! nothing downstream reads the value.

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
            Ok(sigil_link::flatten(&linked, 0x00).expect("flatten"))
        }
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

fn m68k(tail: &str) -> String {
    format!("        cpu 68000\n        padding off\n        phase 0\n{tail}")
}

fn image(tail: &str) -> Vec<u8> {
    assemble(&m68k(tail)).unwrap_or_else(|e| panic!("expected an assembly, got: {e:?}"))
}

fn refusal(tail: &str) -> String {
    match assemble(&m68k(tail)) {
        Ok(bytes) => panic!(
            "assembled to {} byte(s) {:02X?} instead of refusing",
            bytes.len(),
            &bytes[..bytes.len().min(16)]
        ),
        Err(diags) => diags.join(" | "),
    }
}

/// The control. Without it every refusal below could be the harness failing to
/// assemble anything at all, and every acceptance could be a `dc` that emits
/// nothing.
#[test]
fn the_undecorated_forms_still_assemble() {
    assert_eq!(image("        dc.b $11,$22\n"), vec![0x11, 0x22]);
    assert_eq!(image("        dc.w $1234\n"), vec![0x12, 0x34]);
    assert_eq!(image("        dc.l $AABBCCDD\n"), vec![0xAA, 0xBB, 0xCC, 0xDD]);
}

/// d1. The count repeats the value and advances the location counter by the
/// whole run, at each of the three widths.
#[test]
fn a_count_repeats_the_value_at_every_dc_width() {
    assert_eq!(image("        dc.b [3]$FF\n"), vec![0xFF; 3]);
    assert_eq!(
        image("        dc.w [2]$1234\n"),
        vec![0x12, 0x34, 0x12, 0x34]
    );
    assert_eq!(
        image("        dc.l [2]$AABBCCDD\n"),
        vec![0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB, 0xCC, 0xDD]
    );
    // The `$` half of d1: asl puts `here_b` at $1003, `here_w` at $1008 and
    // `here_l` at $1010 for an origin of $1000, i.e. offsets 3, 8 and 16.
    assert_eq!(
        image(
            "        dc.b [3]$FF\n\
             here_b: dc.b $11\n\
                     dc.w [2]$1234\n\
             here_w: dc.l [2]$AABBCCDD\n\
             here_l: dc.b here_b,here_w,here_l\n"
        ),
        vec![
            0xFF, 0xFF, 0xFF, 0x11, 0x12, 0x34, 0x12, 0x34, 0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB,
            0xCC, 0xDD, 3, 8, 16
        ]
    );
}

/// d2. The group is the prefix of ONE operand, so it composes with the rest of
/// the comma list rather than taking the whole line.
#[test]
fn a_count_group_is_one_operand_among_others() {
    assert_eq!(
        image("        dc.b $01,[3]$FF,$02\n"),
        vec![0x01, 0xFF, 0xFF, 0xFF, 0x02]
    );
    assert_eq!(
        image("        dc.b [2]$AA,[2]$BB\n"),
        vec![0xAA, 0xAA, 0xBB, 0xBB]
    );
    assert_eq!(
        image("        dc.w $0001,[2]$1234,$0002\n"),
        vec![0x00, 0x01, 0x12, 0x34, 0x12, 0x34, 0x00, 0x02]
    );
}

/// d3 and d4. The count is an ordinary constant expression, and the whitespace
/// around the group carries no meaning.
#[test]
fn a_count_may_be_an_expression_or_an_earlier_symbol() {
    assert_eq!(image("        dc.b [1+2]$AA\n"), vec![0xAA; 3]);
    assert_eq!(image("        dc.b [(2*2)]$BB\n"), vec![0xBB; 4]);
    assert_eq!(
        image("n:      equ 4\n        dc.b [n]$CC\n        dc.b [n-1]$DD\n"),
        vec![0xCC, 0xCC, 0xCC, 0xCC, 0xDD, 0xDD, 0xDD]
    );
    assert_eq!(image("        dc.b [ 3 ] $AA\n"), vec![0xAA; 3]);
}

/// d5. A zero count emits nothing and does not move `$`: the `$11` and the
/// `$22` end up adjacent.
#[test]
fn a_zero_count_emits_nothing() {
    assert_eq!(
        image("        dc.b $11,[0]$FF,$22\n"),
        vec![0x11, 0x22],
        "a `[0]` operand must vanish, leaving neither a byte nor a gap"
    );
}

/// d7. A string value repeats whole, not one character per repetition.
#[test]
fn a_string_value_repeats_whole() {
    assert_eq!(
        image("        dc.b [2]\"ab\"\n"),
        vec![0x61, 0x62, 0x61, 0x62]
    );
}

/// d13. `MacroSetup.asm`'s own shape, reaching the group through a macro
/// parameter and a `.ATTRIBUTE` width. This is the line all 18 of Sonic 1's
/// `dcb.b` call sites expand into.
#[test]
fn the_macrosetup_dcb_macro_shape_expands() {
    let src = "dcb:    macro count,value\n\
               \x20       dc.ATTRIBUTE [count]value\n\
               \x20       endm\n\
               \x20       dcb.b 3,$FF\n\
               \x20       dcb.w 2,$1234\n\
               \x20       dcb.b 4*2,$20\n";
    let mut want = vec![0xFF, 0xFF, 0xFF, 0x12, 0x34, 0x12, 0x34];
    want.extend(std::iter::repeat_n(0x20u8, 8));
    assert_eq!(image(src), want);
}

/// d17. `MacroSetup.asm` carries the comment "AS can only generate 1 kb of code
/// on a single line" and chunks its `org0` fill at 1024, but the reference build
/// assembles 1025, 1596 and $62A on one line with exit 0, and Sonic 1's own
/// `sonic.asm` writes `dcb.b $62A,$FF`. No ceiling is imposed here either.
#[test]
fn a_count_past_the_1kb_comment_still_assembles() {
    assert_eq!(image("        dc.b [1596]$FD\n"), vec![0xFD; 1596]);
    assert_eq!(image("        dc.b [$62A]$FC\n"), vec![0xFC; 0x62A]);
}

/// d6, and the THIRD divergence. asl reports `expression must be evaluatable in
/// first pass` for a count whose symbol is defined below it. This front end
/// assembles to convergence rather than in one pass, so the count resolves and
/// the run emits the bytes the source means. Pinned as bytes, not as a refusal:
/// the acceptance is a superset of asl's and the VALUE is the point, because
/// the failure mode worth guarding is a count that quietly reads as something
/// else.
#[test]
fn a_forward_referenced_count_resolves_here_where_asl_refuses() {
    assert_eq!(image("        dc.b [fwd]$AA\nfwd:    equ 3\n"), vec![0xAA; 3]);
}

/// A count that never resolves must be REPORTED. `eval_all` answers a poisoned
/// expression with `None` and no diagnostic of its own, so without a word here
/// the operand would emit nothing at all and the run would exit 0: an assembly
/// silently short by however many bytes the count was worth.
#[test]
fn an_unresolvable_count_is_refused_rather_than_dropped() {
    let got = refusal("        dc.b [nosuchsym]$AA\n");
    assert!(
        got.contains("unresolved duplicate count") || got.contains("nosuchsym"),
        "an unresolvable count must be named, not silently dropped. Got: {got}"
    );
}

/// d10. asl reaches the same refusal from the other side, reading the count
/// unsigned and running out of address space (`code overflow`).
#[test]
fn a_negative_count_is_refused_by_name() {
    let got = refusal("        dc.b [-1]$FF\n");
    assert!(
        got.contains("duplicate count -1 out of range"),
        "refused, but not for the count. Got: {got}"
    );
}

/// A DIVERGENCE, pinned so that reversing it is deliberate: asl emits `00 00 00`
/// for `dc.b [3]` (probe d15) and this front end refuses the missing value.
#[test]
fn a_count_with_no_value_is_refused_by_name() {
    let got = refusal("        dc.b [3]\n");
    assert!(
        got.contains("needs a value after the `]`"),
        "refused, but not for the missing value. Got: {got}"
    );
    let got = refusal("        dc.b [3\n");
    assert!(
        got.contains("unterminated `[count]`"),
        "refused, but not for the unterminated group. Got: {got}"
    );
}

/// d9 and d12. The lexer now has `[` and `]` in its alphabet, and this is the
/// guard that they gained no meaning outside a `dc` operand's leading position.
/// asl refuses both of these too, reporting the bracket text as an undefined
/// SYMBOL.
#[test]
fn a_bracket_anywhere_else_is_still_refused() {
    let got = refusal("        move.w #[2]1,d0\n");
    assert!(
        !got.is_empty(),
        "a bracket in an instruction operand must not assemble"
    );
    let got = refusal("        dc.b [2][3]$FF\n");
    assert!(
        !got.is_empty(),
        "only a LEADING bracket is a count; the second group is expression text and does not resolve"
    );
}

/// l1 and l4. `listing` and `page` are listing-file controls, accepted on both
/// CPU surfaces and emitting nothing. The `dc.b` on either side of them is what
/// makes an accepted-but-swallowing implementation visible.
#[test]
fn listing_and_page_are_accepted_on_both_cpus_and_emit_nothing() {
    assert_eq!(
        image(
            "        dc.b $11\n\
             \x20       listing purecode\n\
             \x20       page 0\n\
             \x20       dc.b $22\n"
        ),
        vec![0x11, 0x22]
    );
    let z80 = "        cpu z80\n        phase 0\n\
               \x20       db 11h\n\
               \x20       listing purecode\n\
               \x20       page 0\n\
               \x20       db 22h\n";
    assert_eq!(
        assemble(z80).unwrap_or_else(|e| panic!("expected an assembly, got: {e:?}")),
        vec![0x11, 0x22]
    );
}

/// l3. asl checks the arity of both, and so does this front end. The vocabulary
/// is deliberately not checked; see the module header.
#[test]
fn a_bare_listing_or_page_is_refused_by_name() {
    let got = refusal("        listing\n");
    assert!(
        got.contains("`listing` needs an argument"),
        "refused, but not for the missing argument. Got: {got}"
    );
    let got = refusal("        page\n");
    assert!(
        got.contains("`page` needs an argument"),
        "refused, but not for the missing argument. Got: {got}"
    );
}

/// A column-0 `listing` is the directive, not a label. Without `listing` in the
/// keyword set the bare-label rule would bind a symbol named `listing` and emit
/// nothing, with no diagnostic, which is the shape a dispatch-only fix leaves.
#[test]
fn a_column_zero_listing_is_the_directive_not_a_label() {
    assert_eq!(
        image("dc.b $11\nlisting purecode\ndc.b $22\n"),
        vec![0x11, 0x22]
    );
}
