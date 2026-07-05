//! Integration tests for struct layout checks (Spec 2, Plan 3 — T3):
//! `sizeof`/`offsetof` evaluation, `(size: N)` verification with a
//! field-by-field diff, `@offset` field assertions, and the `[layout.odd-field]`
//! warning. Builds directly on T2's layout engine (`layout.rs`).
use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::layout::{layout_struct, layout_structs_shared};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;
use sigil_span::{Diagnostic, Level};

/// Parse `src` (asserting a clean parse) and evaluate the const named `name`.
fn eval(src: &str, name: &str) -> (Option<Value>, Vec<Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_const(&file, name)
}

fn int(n: i128) -> Value {
    Value::Int(n)
}

// ---- sizeof -------------------------------------------------------------

#[test]
fn sizeof_primitive() {
    let (v, diags) = eval("module m\nconst N = sizeof(u32)\n", "N");
    assert_eq!(v, Some(int(4)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn sizeof_struct() {
    // Fields ordered so every 2/4-byte field lands at an even offset — this
    // test is about `sizeof`, not the `[layout.odd-field]` lint (see the
    // dedicated odd-field tests below), so it stays diagnostic-free.
    let src = "module m\nstruct S { a: u32, b: u16, c: u8 }\nconst N = sizeof(S)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(int(7)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn sizeof_array() {
    let (v, diags) = eval("module m\nconst N = sizeof([u16; 3])\n", "N");
    assert_eq!(v, Some(int(6)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- offsetof -------------------------------------------------------------

#[test]
fn offsetof_known_field() {
    // a@0, b@4, c@8 (declaration-order, no padding; every word/long field at
    // an even offset so this `offsetof`-focused test stays diagnostic-free —
    // see the dedicated odd-field tests below for that lint).
    let src = "module m\nstruct S { a: u32, b: u32, c: u8 }\nconst N = offsetof(S, c)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(int(8)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn offsetof_unknown_field_is_diagnosed() {
    let src = "module m\nstruct S { a: u8 }\nconst N = offsetof(S, nope)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("offsetof") && diags[0].message.contains("nope"),
        "was {:?}",
        diags[0].message
    );
}

#[test]
fn offsetof_non_struct_is_diagnosed() {
    let src = "module m\nconst N = offsetof(u32, nope)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("not a struct"),
        "was {:?}",
        diags[0].message
    );
}

// ---- (size:) verification --------------------------------------------

#[test]
fn declared_size_matching_computed_is_silent() {
    // a@0 (1) + b@1 (2) + c@3 (1) = 4, matches the declared size — no
    // size-mismatch (ERROR) diagnostic. Note: b's 2-byte field at offset 1 IS
    // separately flagged by the independent `[layout.odd-field]` WARNING lint
    // (see the odd-field tests below) — that is a different check and does not
    // make this a size-mismatch.
    let src = "module m\nstruct SeqChannel (size: 4) { a: u8, b: u16, c: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "SeqChannel");
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "expected no size-mismatch (error) diagnostic, got {diags:?}"
    );
    assert!(
        diags.iter().all(|d| !d.message.contains("declared size")),
        "expected no size-mismatch diagnostic, got {diags:?}"
    );
    assert_eq!(layout.expect("SeqChannel should lay out").size, 4);
}

#[test]
fn declared_size_mismatch_is_one_diagnostic_naming_fields_and_delta() {
    let src = "module m\nstruct SeqChannel (size: 58) { a: u8, b: u16, c: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "SeqChannel");
    // Computed total is still surfaced (4), even though it disagrees with the
    // declared size — the raw layout is not poisoned by a size mismatch.
    assert_eq!(layout.expect("SeqChannel should still lay out").size, 4);
    let size_diags: Vec<&Diagnostic> =
        diags.iter().filter(|d| d.message.contains("declared size")).collect();
    assert_eq!(size_diags.len(), 1, "expected exactly one size-mismatch diagnostic, got {diags:?}");
    let msg = &size_diags[0].message;
    assert!(msg.contains("SeqChannel"), "was {msg:?}");
    assert!(msg.contains("declared size 58"), "was {msg:?}");
    assert!(msg.contains("fields total 4"), "was {msg:?}");
    // Names every field so the author can find which one is wrong.
    assert!(msg.contains('a'), "was {msg:?}");
    assert!(msg.contains('b'), "was {msg:?}");
    assert!(msg.contains('c'), "was {msg:?}");
    // Names the delta.
    assert!(msg.contains("off by"), "was {msg:?}");
    assert!(msg.contains("-54") || msg.contains("54"), "was {msg:?}");
}

// ---- @offset field assertions ------------------------------------------

#[test]
fn correct_at_offset_assertion_is_silent() {
    // a: u32 @ 0 keeps b's computed offset (4) even, so this `@offset`-focused
    // test stays diagnostic-free (no incidental odd-field warning).
    let src = "module m\nstruct S { a: u32 @ 0, b: u16 @ 4 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(layout.expect("S should lay out").fields[1].offset, 4);
}

#[test]
fn wrong_at_offset_assertion_is_diagnosed() {
    let src = "module m\nstruct S { a: u32 @ 0, b: u16 @ 99 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    // Layout still computes the real offset (4), independent of the wrong
    // assertion.
    assert_eq!(layout.expect("S should still lay out").fields[1].offset, 4);
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    assert!(
        diags.iter().any(|d| {
            d.message.contains("field b")
                && d.message.contains("offset 4")
                && d.message.contains("asserts 99")
        }),
        "expected an offset-mismatch diagnostic, got {diags:?}"
    );
}

// ---- [layout.odd-field] warning ----------------------------------------

#[test]
fn odd_offset_word_field_is_a_warning() {
    // a: u8 @ 0 (1 byte), b: u16 @ 1 (2-byte field at an ODD offset).
    let src = "module m\nstruct S { a: u8, b: u16 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    let odd: Vec<&Diagnostic> = diags.iter().filter(|d| d.message.contains("odd-field")).collect();
    assert_eq!(odd.len(), 1, "expected exactly one odd-field diagnostic, got {diags:?}");
    assert_eq!(odd[0].level, Level::Warning, "odd-field must be a WARNING, got {:?}", odd[0].level);
    assert!(odd[0].message.contains("field b"), "was {:?}", odd[0].message);
    assert!(odd[0].message.contains("odd offset 1"), "was {:?}", odd[0].message);
}

#[test]
fn aligned_fields_have_no_odd_field_warning() {
    let src = "module m\nstruct S { a: u16, b: u16 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn repeated_query_on_shared_evaluator_does_not_re_emit_odd_field_warning() {
    // The odd-field (and size/@offset) checks run once, on the raw layout,
    // right before it is memoized — a second query for the SAME struct on a
    // shared evaluator must hit the memo and return early, not re-run (and
    // re-warn).
    let src = "module m\nstruct S { a: u8, b: u16 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layouts, diags) = layout_structs_shared(&file, &["S", "S"]);
    assert_eq!(layouts.len(), 2);
    let odd: Vec<&Diagnostic> = diags.iter().filter(|d| d.message.contains("odd-field")).collect();
    assert_eq!(
        odd.len(),
        1,
        "expected the odd-field warning to fire exactly once across two queries, got {diags:?}"
    );
}

// ---- cycles still report only the cycle diagnostic ---------------------

#[test]
fn cyclic_struct_with_declared_size_reports_only_the_cycle() {
    // `Node` both self-references by value AND declares a `(size:)` that
    // could otherwise mismatch — the cycle diagnostic must be the ONLY one;
    // no size-mismatch noise piled on top of an already-poisoned layout.
    let src = "module m\nstruct Node (size: 4) { next: Node }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "Node");
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic struct layout")),
        "expected a cyclic-layout diagnostic, got {diags:?}"
    );
    assert_eq!(diags.len(), 1, "expected only the cycle diagnostic, got {diags:?}");
    let layout = layout.expect("Node should return a (poisoned) layout");
    assert_eq!(layout.size, 0);
    assert!(layout.fields.is_empty());
}

#[test]
fn shared_evaluator_cycle_member_direct_query_has_no_extra_diagnostics() {
    // Regression for the shared-evaluator memo path (mirrors T2's
    // `shared_evaluator_poisons_every_struct_on_the_cycle`): a direct query
    // for a cycle member on the SAME evaluator returns the memoized poison
    // without re-running (and re-diagnosing) any T3 check.
    let src = "module m\nstruct A (size: 4) { b: B }\nstruct B (size: 4) { a: A }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layouts, diags) = layout_structs_shared(&file, &["A", "B"]);
    assert_eq!(
        diags.iter().filter(|d| d.message.contains("cyclic struct layout")).count(),
        1,
        "expected exactly one cycle diagnostic, got {diags:?}"
    );
    assert_eq!(diags.len(), 1, "expected only the cycle diagnostic, got {diags:?}");
    assert!(layouts[0].as_ref().expect("A layout").fields.is_empty());
    assert!(layouts[1].as_ref().expect("B layout").fields.is_empty());
}
