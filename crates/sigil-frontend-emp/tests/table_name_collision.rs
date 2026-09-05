//! `[table.name-collision]` (D2 / S2-D12(d)) — two ordinal tables under one name.
//!
//! `enum`, `offsets`, `dispatch` and `table` all answer `Name.Member`, and
//! `eval_path` tries them in that fixed order. Before this check, declaring two
//! of them under one name resolved SILENTLY to the earlier kind: `dispatch T`'s
//! members reported as "offsets `T` has no member `X`", naming a table the
//! author never asked about, and the pair only collided later, in the linker's
//! whole-program duplicate-label pass (recorded as a sanctioned note at the #6
//! merge, 2026-07-08 — never pinned). Both facts are pinned here.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level};

const LINT: &str = "[table.name-collision]";

fn lower(src: &str) -> Vec<Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    diags
}

fn firings(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| d.message.contains(LINT)).collect()
}

/// The headline pair: `offsets T` + `dispatch T`. One error at declaration,
/// naming both kinds and saying which one wins.
#[test]
fn offsets_and_dispatch_of_one_name_collide() {
    let diags = lower(
        "\
module m
offsets T { A: a }
dispatch T (encoding: word_offsets) { X: a, Y: b }
proc a () { rts }
proc b () { rts }
",
    );
    let f = firings(&diags);
    assert_eq!(f.len(), 1, "exactly one collision report: {diags:?}");
    assert_eq!(f[0].level, Level::Error, "a pair that also fails the link is error tier");
    assert!(f[0].message.contains("`T`"), "names the table: {}", f[0].message);
    assert!(f[0].message.contains("offsets"), "names one kind: {}", f[0].message);
    assert!(f[0].message.contains("dispatch"), "names the other kind: {}", f[0].message);
    assert!(
        f[0].message.contains("against the `offsets` table"),
        "names the WINNER: {}",
        f[0].message
    );
}

/// The winner is the LADDER rank, not the declaration order. Declaring the
/// `dispatch` FIRST does not make it win — `eval_path` still tries offsets
/// before dispatch — so the message must still say `offsets`. (This is the
/// order half of the pair above; asserting only the ladder-ordered spelling
/// would leave the reversal untested and the message wrong half the time.)
#[test]
fn the_named_winner_follows_the_ladder_not_the_declaration_order() {
    let diags = lower(
        "\
module m
dispatch T (encoding: word_offsets) { X: a }
offsets T { A: a }
proc a () { rts }
",
    );
    let f = firings(&diags);
    assert_eq!(f.len(), 1, "firings: {diags:?}");
    assert!(
        f[0].message.contains("against the `offsets` table"),
        "offsets wins regardless of who was written first: {}",
        f[0].message
    );
}

/// The behavior the collision USED to produce, still visible alongside the new
/// error — proof this is the real bug and not a hypothetical: `T.X` is a
/// `dispatch` member, and the ordinal lookup answers for the `offsets` table.
#[test]
fn the_shadowed_member_still_misreports_which_table_it_checked() {
    let diags = lower(
        "\
module m
offsets T { A: a }
dispatch T (encoding: word_offsets) { X: a }
proc a () { rts }
data ids: [u8; 1] = [T.X]
",
    );
    assert!(
        diags.iter().any(|d| d.message.contains("offsets `T` has no member `X`")),
        "the silent-precedence symptom the lint explains: {diags:?}"
    );
    assert_eq!(firings(&diags).len(), 1, "and the declaration-site error names it");
}

/// The same kind twice is the identical hazard (a map insert — last wins), and
/// the message says so rather than inventing a second kind.
#[test]
fn the_same_kind_twice_collides_too() {
    let diags = lower(
        "\
module m
offsets T { A: a }
offsets T { B: a }
proc a () { rts }
",
    );
    let f = firings(&diags);
    assert_eq!(f.len(), 1, "firings: {diags:?}");
    assert!(f[0].message.contains("twice"), "phrased for one kind: {}", f[0].message);
}

/// `enum` sits on the same `Name.Member` ladder, so it collides too — but at
/// WARN tier, in both declaration orders. An `enum` emits no base label, so an
/// `enum`+`offsets` module links and runs today; refusing it would break a legal
/// program to report a readability hazard. The enum always wins the lookup
/// (ladder rank 0), whichever way round it is written.
#[test]
fn an_enum_pair_is_warn_tier_in_both_orders() {
    for src in [
        "\
module m
enum K: u8 { A = 0, B = 1 }
offsets K { X: a }
proc a () { rts }
",
        "\
module m
offsets K { X: a }
enum K: u8 { A = 0, B = 1 }
proc a () { rts }
",
    ] {
        let diags = lower(src);
        let f = firings(&diags);
        assert_eq!(f.len(), 1, "enum vs offsets collides: {diags:?}");
        assert_eq!(
            f[0].level,
            Level::Warning,
            "an enum carries no base label, this program links today: {}",
            f[0].message
        );
        assert!(
            f[0].message.contains("against the `enum` table"),
            "the enum wins the ladder either way: {}",
            f[0].message
        );
    }
}

/// `table` is the ladder's LAST rung, so it never wins against a sibling.
#[test]
fn table_never_wins_the_ladder() {
    let diags = lower(
        "\
module m
table T (key: 0..=1) { 0: [1] }
dispatch T (encoding: word_offsets) { X: a }
proc a () { rts }
",
    );
    let f = firings(&diags);
    assert_eq!(f.len(), 1, "firings: {diags:?}");
    assert_eq!(f[0].level, Level::Error, "both emit a base label, error tier");
    assert!(
        f[0].message.contains("against the `dispatch` table"),
        "dispatch outranks table: {}",
        f[0].message
    );
}

/// Section nesting does not hide a collision — the item namespace is flat
/// (§7.1), exactly as every sibling validator treats it.
#[test]
fn a_section_nested_declaration_still_collides() {
    let diags = lower(
        "\
module m
offsets T { A: a }
section rom {
    dispatch T (encoding: word_offsets) { X: a }
}
proc a () { rts }
",
    );
    assert_eq!(firings(&diags).len(), 1, "flat namespace: {diags:?}");
}

/// Distinct names are the normal case and stay silent — the check is not just
/// "two tables in one module".
#[test]
fn distinct_names_are_silent() {
    let diags = lower(
        "\
module m
offsets Ani { A: a }
dispatch Routines (encoding: word_offsets) { X: a }
table Rows (key: 0..=1) { 0: [1] }
enum K: u8 { Q = 0 }
proc a () { rts }
",
    );
    assert!(firings(&diags).is_empty(), "no collision: {diags:?}");
}
