//! `[name.duplicate-const]` (A10, W13): a `const` declared twice in one module is refused.
//! Before, the evaluator's index kept the last one.
use sigil_frontend_emp::parse_str;
use sigil_span::Level;

const ID: &str = "[name.duplicate-const]";

fn refusals(src: &str) -> Vec<sigil_span::Diagnostic> {
    parse_str(src).1.into_iter().filter(|d| d.level == Level::Error && d.message.contains(ID)).collect()
}

#[test]
fn a_const_declared_twice_is_refused_at_the_later_one() {
    let src = "module m\nconst Foo = 1\nconst Foo = 2\n";
    let r = refusals(src);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].message.contains("constant `Foo` is declared twice"), "{}", r[0].message);
    assert_eq!(r[0].primary.start as usize, src.rfind("const Foo").unwrap(), "anchored at the later declaration");
}

#[test]
fn pub_typed_and_section_nested_duplicates_are_refused() {
    assert_eq!(refusals("module m\npub const Foo = 1\nconst Bar = 2\nconst Foo: u8 = 3\n").len(), 1);
    assert_eq!(refusals("module m\nconst Foo = 1\nsection text {\n    const Foo = 2\n}\n").len(), 1);
}

/// Controls: distinct names; a const beside an enum, a fn and an imported name.
#[test]
fn distinct_names_and_other_kinds_stay_legal() {
    assert!(refusals("module m\nconst Foo = 1\nconst Bar = 2\n").is_empty());
    assert!(refusals("module m\nconst Foo = 1\nenum Foo: u8 { A }\ncomptime fn Bar() -> int {\n    return 1\n}\nconst Baz = 2\n").is_empty());
    assert!(refusals("module m\nuse other.{Foo}\nconst Foo = 1\n").is_empty());
}

/// The limit of this task: `const` against `equ` is not refused here (booked in docs/QUEUE.md).
#[test]
fn a_const_beside_an_equ_of_the_same_name_is_not_this_refusal() {
    assert!(refusals("module m\nconst Foo = 1\nequ Foo = 2\n").is_empty());
}
