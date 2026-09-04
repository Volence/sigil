// The `text` block in the module doc below is an asl listing pasted verbatim, and asl
// separates its columns with TABS. Those tabs are the evidence: what that comment asserts
// is what the reference assembler PRINTED, so respacing them into four spaces would
// quietly restate the claim about output asl never produced.
//
// This one is an INNER attribute, unlike the per-item waivers in the sibling test files,
// and the difference is forced rather than chosen: the listing lives in the crate-root
// `//!` doc, and Rust has no narrower scope than the module for a lint fired there. The
// blast radius is this single integration-test binary, whose only other doc lines are
// prose - it is not a crate-wide allow over library code, which would be trading a
// correct lint everywhere for one site.
#![allow(clippy::tabs_in_doc_comments)]

//! A macro call may not write a positional argument after a keyword one, and a
//! keyword argument must name a parameter the macro declares.
//!
//! AS refuses both (`#1812 positional argument no longer allowed after keyword
//! argument`, `#1811 keyword argument not defined in macro`). Accepting them is
//! not a difference in how the aftermath READS: asl binds the refused argument
//! to NOTHING, so a call this front end accepted assembled the macro body
//! against arguments asl never supplied — a wrong program with no diagnostic
//! anywhere in it.
//!
//! Every expectation below is read off `asl` 1.42 Beta Bld 212's own listing for
//! the identical source, run with Sonic 1's own flags (`asl -xx -n -q -A -L -U
//! -i . probe.asm`, as `s1disasm/build_tools/lua/common.lua:773` invokes it).
//! Over `m macro px,py,pz` / `dc.b px,py,pz`:
//!
//! ```text
//!    6/       0 : (MACRO)              	m	1,2,3
//!    6/       0 : 0102 03                     dc.b    1,2,3
//!    7/       3 : (MACRO)              	m	px=1,py=2,pz=3
//!    7/       3 : 0102 03                     dc.b    1,2,3
//!   > > > k2.asm(8): error #1812: positional argument no longer allowed after keyword argument
//!   > > >  m py=$22,1,3
//!   > > > k2.asm(8): error #1812: positional argument no longer allowed after keyword argument
//!   > > >  m py=$22,1,3
//!    8/       6 : (MACRO)              	m	py=$22,1,3
//!    8/       6 :                             dc.b    ,$22,
//!   > > > k2.asm(9): error #1812: positional argument no longer allowed after keyword argument
//!   > > >  m 1,py=$22,3
//!    9/       6 : (MACRO)              	m	1,py=$22,3
//!    9/       6 :                             dc.b    1,$22,
//!   > > > k3.asm(7):6: error #1811: keyword argument not defined in macro
//!   > > > zz
//!   > > >  m 1,zz=2,3
//!    7/       0 : (MACRO)              	m	1,zz=2,3
//!    7/       0 :                             dc.b    1,,
//!   > > > k6.asm(10):6: error #1811: keyword argument not defined in macro
//!   > > >
//!   > > >  m 1,=5,4
//!    7/       0 : (MACRO)              	m	1,(2=3),4
//!    7/       0 : 0100 04                     dc.b    1,(2=3),4
//!    8/       0 : (MACRO)              	m	1,"py=2",3
//!    8/       0 : 0170 793D 3203              dc.b    1,"py=2",3
//!    9/       3 : (MACRO)              	m	1,py=,4
//!    9/       3 :                             dc.b    1,,
//! ```
//!
//! Read off those rows: #1812 fires ONCE PER offending positional (two on
//! `m py=$22,1,3`, one on `m 1,py=$22,3`); an unknown keyword name still ARMS
//! the rule for what follows it; a keyword written with an empty value is a real
//! binding to empty text rather than a positional; and a `=` inside parentheses
//! or a string literal is an ordinary expression, not a separator.

use sigil_frontend_as::{assemble, Options};

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\tphase 0\n";
const MDEF: &str = "m\tmacro\tpx,py,pz\n\tdc.b\tpx,py,pz\n\tendm\n";

/// The diagnostic messages for one call, or `Ok` with the linked image when the
/// call assembles. A byte assertion is what proves an ACCEPTED call still binds
/// what it always bound; a message assertion is what proves a refused one is
/// refused for the stated reason.
fn run(call: &str) -> Result<Vec<u8>, Vec<String>> {
    let src = format!("{HEAD}{MDEF}{call}");
    let m = assemble(&src, &Options::default())
        .map_err(|d| d.iter().map(|x| x.message.clone()).collect::<Vec<_>>())?;
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .unwrap_or_else(|e| panic!("did not lay out: {e:?}\n{src}"));
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
        .unwrap_or_else(|e| panic!("did not link: {e:?}\n{src}"));
    Ok(sigil_link::flatten(&linked, 0x00))
}

/// How many diagnostics carry `needle`, so a count assertion says WHICH message
/// it counted — a bare total cannot tell two refusals of one call apart from one
/// refusal and an unrelated error.
fn count(diags: &[String], needle: &str) -> usize {
    diags.iter().filter(|d| d.contains(needle)).count()
}

const POSITIONAL_AFTER_KEYWORD: &str = "positional argument no longer allowed after keyword argument";
const UNDEFINED_KEYWORD: &str = "not defined in macro";

/// asl listing rows 6 and 7: an all-positional call and an all-keyword call both
/// emit `01 02 03`. The refusal must not reach either.
#[test]
fn well_formed_calls_still_assemble() {
    assert_eq!(run("\tm\t1,2,3\n").expect("all-positional call refused"), vec![1, 2, 3]);
    assert_eq!(
        run("\tm\tpx=1,py=2,pz=3\n").expect("all-keyword call refused"),
        vec![1, 2, 3]
    );
    // Keyword arguments out of declaration order still bind by name.
    assert_eq!(run("\tm\tpz=3,px=1,py=2\n").expect("out-of-order keyword call refused"), vec![1, 2, 3]);
    // Positionals BEFORE the first keyword are fine — the rule is about after.
    assert_eq!(run("\tm\t1,2,pz=3\n").expect("leading positionals refused"), vec![1, 2, 3]);
}

/// asl listing row 8: TWO #1812s for `m py=$22,1,3`, because two positional
/// arguments follow the keyword. One diagnostic for two mistakes would under-report.
#[test]
fn refuses_each_positional_after_a_keyword() {
    let d = run("\tm\tpy=$22,1,3\n").expect_err("accepted a positional after a keyword");
    assert_eq!(
        count(&d, POSITIONAL_AFTER_KEYWORD),
        2,
        "expected asl's two #1812s, got {d:?}"
    );
}

/// asl listing row 9: ONE #1812 for `m 1,py=$22,3` — the leading `1` precedes the
/// keyword and is legal, only the trailing `3` is refused.
#[test]
fn refuses_only_the_positionals_that_follow_the_keyword() {
    let d = run("\tm\t1,py=$22,3\n").expect_err("accepted a positional after a keyword");
    assert_eq!(
        count(&d, POSITIONAL_AFTER_KEYWORD),
        1,
        "expected asl's single #1812, got {d:?}"
    );
}

/// asl `k3.asm(7)`: `zz` is not a parameter, so the keyword itself is refused
/// (#1811) AND the rule is armed, refusing the `3` behind it. An unknown keyword
/// that silently became a positional would bind `zz=2` to `py` — asl binds nothing.
#[test]
fn refuses_a_keyword_naming_no_parameter_and_still_arms_the_rule() {
    let d = run("\tm\t1,zz=2,3\n").expect_err("accepted an undefined keyword argument");
    assert_eq!(count(&d, UNDEFINED_KEYWORD), 1, "expected asl's #1811, got {d:?}");
    assert!(
        d.iter().any(|m| m.contains("`zz`")),
        "the refusal must name the keyword it could not find: {d:?}"
    );
    assert_eq!(
        count(&d, POSITIONAL_AFTER_KEYWORD),
        1,
        "an unknown keyword still arms #1812 for what follows it: {d:?}"
    );
}

/// asl `k4.asm(10)` under `-U`: `PX` is not `px`, so a case-mismatched keyword is
/// #1811 and both arguments behind it are #1812. Two, not one — the count is the
/// assertion that each offending argument was seen.
#[test]
fn keyword_names_are_case_sensitive() {
    let d = run("\tm\tPX=1,2,3\n").expect_err("accepted a case-mismatched keyword");
    assert_eq!(count(&d, UNDEFINED_KEYWORD), 1, "expected asl's #1811, got {d:?}");
    assert_eq!(
        count(&d, POSITIONAL_AFTER_KEYWORD),
        2,
        "expected asl's two #1812s, got {d:?}"
    );
}

/// asl `k6.asm(9)`: `py=` binds `py` to empty text — a keyword, not a positional
/// — so the `4` behind it is refused. Treating an empty value as "not a keyword"
/// would leave this call silently accepted with `4` bound to `pz`.
#[test]
fn a_keyword_with_an_empty_value_is_still_a_keyword() {
    let d = run("\tm\t1,py=,4\n").expect_err("accepted a positional after an empty keyword");
    assert_eq!(
        count(&d, POSITIONAL_AFTER_KEYWORD),
        1,
        "an empty-valued keyword still arms #1812: {d:?}"
    );
    assert_eq!(count(&d, UNDEFINED_KEYWORD), 0, "`py` IS a parameter: {d:?}");
}

/// asl `k6.asm(8)` and `k4.asm(6)`: the split is on the raw `=`, so `2=3` and
/// `2+3=5` are keyword arguments whose NAME is not an identifier at all. The
/// refusal must quote what asl quotes.
#[test]
fn a_non_identifier_before_the_equals_is_still_a_keyword_name() {
    let d = run("\tm\t1,2=3,4\n").expect_err("accepted `2=3` as a positional");
    assert!(
        d.iter().any(|m| m.contains(UNDEFINED_KEYWORD) && m.contains("`2`")),
        "asl names the keyword `2`: {d:?}"
    );
    let d = run("\tm\t1,2+3=5,4\n").expect_err("accepted `2+3=5` as a positional");
    assert!(
        d.iter().any(|m| m.contains(UNDEFINED_KEYWORD) && m.contains("`2+3`")),
        "asl names the keyword `2+3`: {d:?}"
    );
}

/// asl `k5.asm(10)` and `k6.asm(7)`: a `=` inside a string literal or inside
/// parentheses is an ordinary expression. Both calls emit bytes in asl's listing
/// and must keep emitting them here — this is the over-refusal guard.
#[test]
fn an_equals_inside_parentheses_or_a_string_is_not_a_separator() {
    // asl row: `dc.b 1,(2=3),4` → `0100 04`. AS's `=` compares; 2=3 is false.
    assert_eq!(
        run("\tm\t1,(2=3),4\n").expect("refused a parenthesised comparison"),
        vec![1, 0, 4]
    );
    // asl row: `dc.b 1,"py=2",3` → `0170 793D 3203`.
    assert_eq!(
        run("\tm\t1,\"py=2\",3\n").expect("refused a quoted `=`"),
        vec![0x01, 0x70, 0x79, 0x3D, 0x32, 0x03]
    );
}
