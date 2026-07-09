//! `///` doc comments — parse and attach (S2-D11(d), ratified IN at the v1
//! freeze). `///` lines lex as DOC trivia and attach to the item that follows
//! (the run joins with `\n`, one optional leading space stripped per line);
//! a run that precedes no item warns `[doc.dangling]`. No output surface yet
//! (hover is Spec 3) — this suite pins the parse-and-attach layer only.

use sigil_frontend_emp::ast::{item_span, Item};
use sigil_frontend_emp::parse_str;

/// Parse and return the file plus diagnostic messages.
fn parse(src: &str) -> (sigil_frontend_emp::ast::File, Vec<String>) {
    let (f, diags) = parse_str(src);
    (f, diags.into_iter().map(|d| format!("{:?}:{}", d.level, d.message)).collect())
}

#[test]
fn doc_comment_attaches_to_following_item() {
    let src = "\
module m
/// The wait timer's reset value.
const A: u8 = 1
";
    let (f, msgs) = parse(src);
    assert!(msgs.is_empty(), "clean parse: {msgs:?}");
    let span = item_span(&f.items[0]);
    assert_eq!(f.docs_for(span), Some("The wait timer's reset value."));
}

#[test]
fn multi_line_doc_run_joins_with_newlines() {
    let src = "\
module m
/// Line one.
/// Line two.
proc p () {
    rts
}
";
    let (f, msgs) = parse(src);
    assert!(msgs.is_empty(), "clean parse: {msgs:?}");
    let span = item_span(&f.items[0]);
    assert_eq!(f.docs_for(span), Some("Line one.\nLine two."));
}

#[test]
fn doc_before_pub_item_attaches() {
    let src = "\
module m
/// Exported table.
pub const T: u8 = 3
";
    let (f, msgs) = parse(src);
    assert!(msgs.is_empty(), "clean parse: {msgs:?}");
    assert_eq!(f.docs_for(item_span(&f.items[0])), Some("Exported table."));
}

#[test]
fn doc_attaches_inside_section_bodies() {
    let src = "\
module m
section s (vma: $100) {
    /// The section-resident item.
    data D: [u8; 1] = [7]
}
";
    let (f, msgs) = parse(src);
    assert!(msgs.is_empty(), "clean parse: {msgs:?}");
    let Item::Section(sec) = &f.items[0] else { panic!("expected section") };
    assert_eq!(f.docs_for(item_span(&sec.items[0])), Some("The section-resident item."));
}

#[test]
fn undocumented_item_has_no_docs() {
    let src = "\
module m
const A: u8 = 1
";
    let (f, msgs) = parse(src);
    assert!(msgs.is_empty(), "clean parse: {msgs:?}");
    assert_eq!(f.docs_for(item_span(&f.items[0])), None);
}

#[test]
fn dangling_doc_at_eof_warns() {
    let src = "\
module m
const A: u8 = 1
/// Nothing follows me.
";
    let (_, msgs) = parse(src);
    assert!(
        msgs.iter().any(|m| m.contains("[doc.dangling]") && m.starts_with("Warning")),
        "expected a warning-tier [doc.dangling]: {msgs:?}"
    );
}

#[test]
fn dangling_doc_in_proc_body_warns_and_parses_on() {
    // A full-line `///` inside a body is not attached to anything — warn,
    // keep parsing (the surrounding proc stays intact).
    let src = "\
module m
proc p () {
    nop
    /// stray doc line
    rts
}
";
    let (f, msgs) = parse(src);
    assert!(
        msgs.iter().any(|m| m.contains("[doc.dangling]")),
        "expected [doc.dangling]: {msgs:?}"
    );
    assert!(matches!(&f.items[0], Item::Proc(_)), "proc still parses");
}

#[test]
fn four_slashes_is_an_ordinary_comment() {
    let src = "\
module m
//// decorative ruler, not a doc
const A: u8 = 1
";
    let (f, msgs) = parse(src);
    assert!(msgs.is_empty(), "clean parse: {msgs:?}");
    assert_eq!(f.docs_for(item_span(&f.items[0])), None);
}

#[test]
fn ordinary_comments_between_doc_and_item_do_not_detach() {
    let src = "\
module m
/// Documented.
// (implementation note, not docs)
const A: u8 = 1
";
    let (f, msgs) = parse(src);
    assert!(msgs.is_empty(), "clean parse: {msgs:?}");
    assert_eq!(f.docs_for(item_span(&f.items[0])), Some("Documented."));
}

#[test]
fn preview_style_script_doc_attaches() {
    let src = "\
module m
newtype ScriptPc = u16
struct S (size: $24) {
    _pad0: [u8; $20],
    resume: ScriptPc @ $20,
    _pad1: [u8; 2] @ $22,
}
/// The brain, D2.30 style.
/// Yields store the resume word.
script brain (a0: *S) (encoding: word_offsets) shows done {
    yield
}
proc done () { rts }
";
    let (f, msgs) = parse(src);
    assert!(msgs.is_empty(), "clean parse: {msgs:?}");
    let script = f
        .items
        .iter()
        .find(|i| matches!(i, Item::Script(_)))
        .expect("script item");
    assert_eq!(
        f.docs_for(item_span(script)),
        Some("The brain, D2.30 style.\nYields store the resume word.")
    );
}
