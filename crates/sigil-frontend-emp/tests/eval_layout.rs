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
    // Names every field's diff line (name @offset) so the author can find which
    // one is wrong. (Assert the full `x @off` fragment, not a bare char — the
    // struct name "SeqChannel" contains 'a', which would pass vacuously.)
    assert!(msg.contains("a @0"), "was {msg:?}");
    assert!(msg.contains("b @1"), "was {msg:?}");
    assert!(msg.contains("c @3"), "was {msg:?}");
    // Names the delta, absolute + directional (never a bare negative).
    assert!(msg.contains("off by 54"), "was {msg:?}");
    assert!(msg.contains("too small"), "was {msg:?}");
    assert!(!msg.contains("-54"), "delta must be absolute, not negative: {msg:?}");
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

// ---- L4: `@allow("layout.odd-field")` at module scope --------------------

#[test]
fn module_allow_silences_odd_field() {
    // The intentionally-unaligned Z80-record case: the module declares the intent
    // once, at module scope, and the lint goes quiet.
    let src = "module m\n@allow(\"layout.odd-field\")\nstruct S { a: u8, b: u16 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    let odd: Vec<&Diagnostic> = diags.iter().filter(|d| d.message.contains("odd-field")).collect();
    assert!(odd.is_empty(), "@allow(\"layout.odd-field\") must silence the lint: {diags:?}");
}

#[test]
fn allow_of_a_different_lint_does_not_silence_odd_field() {
    // The allow is lint-specific: naming a DIFFERENT id leaves odd-field firing.
    let src = "module m\n@allow(\"layout.odd-item\")\nstruct S { a: u8, b: u16 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    let odd: Vec<&Diagnostic> = diags.iter().filter(|d| d.message.contains("odd-field")).collect();
    assert_eq!(odd.len(), 1, "a different lint's allow must NOT silence odd-field: {diags:?}");
    assert_eq!(odd[0].level, Level::Warning);
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

// ---- re-entrancy: self-referential (size:)/@offset must not crash ------

#[test]
fn self_referential_size_expr_is_cyclic_not_a_crash() {
    // Critical 1 regression: `(size: sizeof(Foo))` inside `Foo` re-enters
    // `layout_of_struct(Foo)` DURING the `(size:)` check. Pre-fix, `Foo` had
    // already been popped from the in-progress stack (and not yet memoized), so
    // the re-entrant call fell through cycle detection into infinite recursion
    // → SIGABRT. It must now report a cyclic-layout diagnostic instead.
    let src = "module m\nstruct Foo (size: sizeof(Foo)) { a: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "Foo");
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic struct layout")),
        "expected a cyclic-layout diagnostic, got {diags:?}"
    );
    // Exactly the cycle diagnostic — no spurious size-mismatch piled on top.
    assert_eq!(diags.len(), 1, "expected only the cycle diagnostic, got {diags:?}");
    let layout = layout.expect("Foo should return a (poisoned) layout");
    assert_eq!(layout.size, 0);
    assert!(layout.fields.is_empty());
}

#[test]
fn mutual_size_sizeof_pair_is_cyclic_not_a_crash() {
    // Critical 1 regression, mutual form: A's `(size: sizeof(B))` check lays out
    // B, whose `(size: sizeof(A))` check re-enters A (still in-progress) and
    // closes the cycle. Pre-fix this recursed forever.
    let src = "module m\n\
               struct A (size: sizeof(B)) { x: u8 }\n\
               struct B (size: sizeof(A)) { y: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "A");
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic struct layout")),
        "expected a cyclic-layout diagnostic, got {diags:?}"
    );
    assert_eq!(diags.len(), 1, "expected only the cycle diagnostic, got {diags:?}");
}

#[test]
fn self_referential_at_offset_expr_is_cyclic_not_a_crash() {
    // Critical 1 regression via `@offset`: `b: u8 @ sizeof(S)` re-enters
    // `layout_of_struct(S)` during the offset check.
    let src = "module m\nstruct S { a: u8, b: u8 @ sizeof(S) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic struct layout")),
        "expected a cyclic-layout diagnostic, got {diags:?}"
    );
    assert_eq!(diags.len(), 1, "expected only the cycle diagnostic, got {diags:?}");
}

// ---- offsetof through a newtype ----------------------------------------

#[test]
fn offsetof_through_newtype_wrapping_a_struct() {
    // Positive coverage for the `struct_name_for_offsetof` Newtype branch: a
    // newtype that wraps a struct resolves offsetof to the field's offset.
    let src = "module m\n\
               struct Inner { a: u8, b: u8 }\n\
               newtype Wrap = Inner\n\
               const N = offsetof(Wrap, b)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(int(1)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn offsetof_newtype_cycle_is_diagnosed_not_a_crash() {
    // Critical 2 regression: `newtype A = B; newtype B = A` gives
    // `struct_name_for_offsetof` no struct to bottom out at; pre-fix its
    // Newtype branch recursed with no cycle guard → SIGABRT. It must now report
    // a diagnostic (a cyclic-type one) and poison instead.
    let src = "module m\n\
               newtype A = B\n\
               newtype B = A\n\
               const N = offsetof(A, x)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(!diags.is_empty(), "expected a diagnostic, got none");
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic type")),
        "expected a cyclic-type diagnostic, got {diags:?}"
    );
}

// ---- `(align: N)` field assertions --------------------------------------
//
// The error-tier, per-field, opt-in counterpart to `[layout.odd-field]`. It
// asserts a PROPERTY of the computed offset, so it survives every legitimate
// insertion above the field, and it is not a lint — no `@allow` reaches it.

#[test]
fn field_align_satisfied_is_silent() {
    // THE CONTROL ARM. head: u16 @0 (2 bytes), so bridge lands at 2, and
    // 2 % 2 == 0. A check that rejected everything would still pass the
    // violation test below; only this one refutes it.
    let src = "module m\nstruct Rec { head: u16, bridge: i16 (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "Rec");
    assert!(diags.is_empty(), "a satisfied (align:) must be silent: {diags:?}");
    assert_eq!(layout.expect("Rec should lay out").fields[1].offset, 2);
}

#[test]
fn field_align_violation_is_an_error() {
    // THE POISON ARM. head: u8 @0 (1 byte), so bridge lands at 1, and 1 % 2 == 1.
    let src = "module m\nstruct Rec { head: u8, bridge: i16 (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "Rec");
    // The layout still reports the real offset — the assertion judges it, it
    // does not move it.
    assert_eq!(layout.expect("Rec should still lay out").fields[1].offset, 1);
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    let d = &diags[0];
    assert_eq!(d.level, Level::Error, "(align:) must be an ERROR, got {:?}", d.level);
    // Names the field, the claim, and the live offset.
    assert!(d.message.contains("field bridge"), "was {:?}", d.message);
    assert!(d.message.contains("declares (align: 2)"), "was {:?}", d.message);
    assert!(d.message.contains("but lands at offset 1"), "was {:?}", d.message);
    // Names the padding delta in both directions: 2 - (1 % 2) to add, 1 % 2 to remove.
    assert!(d.message.contains("add 1 or remove 1"), "was {:?}", d.message);
    // Names the INSTRUCTION that would fault, never an offset to pin.
    assert!(d.message.contains("68000 address error"), "was {:?}", d.message);
    assert!(d.message.contains("Do not pin an @offset"), "was {:?}", d.message);
}

#[test]
fn field_align_error_is_not_silenced_by_the_odd_field_allow() {
    // `@allow("layout.odd-field")` speaks for the warning tier only. An explicit
    // (align:) claim is not a lint and the allow must not reach it — the whole
    // reason the attribute exists is that the warning tier is silenceable.
    let src =
        "module m\n@allow(\"layout.odd-field\")\nstruct Rec { head: u8, bridge: i16 (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Rec");
    let errs: Vec<&Diagnostic> = diags.iter().filter(|d| d.level == Level::Error).collect();
    assert_eq!(errs.len(), 1, "the allow must not silence (align:): {diags:?}");
    assert!(errs[0].message.contains("but lands at offset 1"), "was {:?}", errs[0].message);
}

#[test]
fn field_align_supersedes_the_odd_field_warning() {
    // The explicit claim has already judged this field at the tier the author
    // asked for; the heuristic lint must not double-report the same fault.
    let src = "module m\nstruct Rec { head: u8, bridge: i16 (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Rec");
    let odd: Vec<&Diagnostic> = diags.iter().filter(|d| d.message.contains("odd-field")).collect();
    assert!(odd.is_empty(), "(align:) must supersede the odd-field lint: {diags:?}");
}

#[test]
fn field_align_one_is_the_per_field_odd_field_opt_out() {
    // `(align: 1)` is the identity claim: this word's parity is deliberately not
    // load-bearing. It silences the lint for THIS field, where the module-scope
    // allow can only speak for every field at once.
    let src = "module m\nstruct Rec { head: u8, loose: i16 (align: 1), tail: i16 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Rec");
    // loose@1 is exempt; tail@3 is a bare odd word and still warns — proving the
    // opt-out is per-field and not a module-wide silence.
    let odd: Vec<&Diagnostic> = diags.iter().filter(|d| d.message.contains("odd-field")).collect();
    assert_eq!(odd.len(), 1, "expected only tail to warn, got {diags:?}");
    assert!(odd[0].message.contains("field tail"), "was {:?}", odd[0].message);
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "(align: 1) must never error: {diags:?}"
    );
}

#[test]
fn field_align_even_offset_missing_a_wider_alignment_omits_the_cpu_claim() {
    // head: u16 @0, wide@2. 2 % 4 == 2 — the claim is violated, but 2 is EVEN,
    // so no 68000 access faults here and the message must not say one would.
    let src = "module m\nstruct Rec { head: u16, wide: u32 (align: 4) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Rec");
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    let msg = &diags[0].message;
    assert!(msg.contains("but lands at offset 2"), "was {msg:?}");
    // add 4 - (2 % 4) = 2, remove 2 % 4 = 2.
    assert!(msg.contains("add 2 or remove 2"), "was {msg:?}");
    assert!(!msg.contains("address error"), "must not invent a CPU fault: {msg:?}");
}

#[test]
fn field_align_non_power_of_two_is_refused() {
    let src = "module m\nstruct Rec { head: u8, odd_claim: u16 (align: 3) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Rec");
    let errs: Vec<&Diagnostic> = diags.iter().filter(|d| d.level == Level::Error).collect();
    assert_eq!(errs.len(), 1, "expected exactly one error, got {diags:?}");
    // Matched on the rule-unique `declares (align:` prefix, not on the trailing
    // clause alone — `[section.bank]` also refuses non-powers-of-two and a bare
    // phrase match could be satisfied by an unrelated rule.
    assert_eq!(
        errs[0].message,
        "struct Rec: field odd_claim declares (align: 3) but an alignment must be a power of two (1, 2, 4, 8, ...)",
        "the whole string is the contract, parenthetical included"
    );
    // A malformed alignment is refused, not evaluated — no offset verdict follows.
    assert!(!errs[0].message.contains("lands at offset"), "was {:?}", errs[0].message);
}

#[test]
fn field_align_zero_is_refused_not_a_modulo_by_zero() {
    let src = "module m\nstruct Rec { head: u8, bad: u16 (align: 0) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Rec");
    let errs: Vec<&Diagnostic> = diags.iter().filter(|d| d.level == Level::Error).collect();
    assert_eq!(errs.len(), 1, "expected exactly one error, got {diags:?}");
    assert_eq!(
        errs[0].message,
        "struct Rec: field bad declares (align: 0) but an alignment must be a power of two (1, 2, 4, 8, ...)",
        "the whole string is the contract, parenthetical included"
    );
}

#[test]
fn field_align_takes_a_comptime_expression() {
    // The alignment is an expr, as `@offset` and `(size:)` are — a named
    // constant is as good as a literal.
    let src = "module m\nconst WORD = 2\nstruct Rec { head: u8, bridge: i16 (align: WORD) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Rec");
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    assert!(diags[0].message.contains("declares (align: 2)"), "was {:?}", diags[0].message);
}

#[test]
fn field_align_and_at_offset_are_independent_assertions() {
    // bridge lands at 1. `@ 1` is satisfied; `(align: 2)` is not. Each judges the
    // same computed offset for a different property and reports on its own.
    let src = "module m\nstruct Rec { head: u8, bridge: i16 @ 1 (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Rec");
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    assert!(diags[0].message.contains("but lands at offset 1"), "was {:?}", diags[0].message);
    assert!(!diags[0].message.contains("asserts"), "the @offset held: {:?}", diags[0].message);
}

#[test]
fn field_align_and_a_wrong_at_offset_both_report() {
    // Two different faults on one field: the snapshot is stale AND the parity
    // broke. Both are named; neither suppresses the other.
    let src = "module m\nstruct Rec { head: u8, bridge: i16 @ 99 (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Rec");
    assert_eq!(diags.len(), 2, "expected both assertions to report, got {diags:?}");
    assert!(diags.iter().any(|d| d.message.contains("asserts 99")), "got {diags:?}");
    assert!(diags.iter().any(|d| d.message.contains("but lands at offset 1")), "got {diags:?}");
}

#[test]
fn field_align_accepts_either_attribute_order() {
    // `(align:) @ offset` parses the same as `@ offset (align:)`.
    let src = "module m\nstruct Rec { head: u8, bridge: i16 (align: 2) @ 1 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Rec");
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    assert!(diags[0].message.contains("but lands at offset 1"), "was {:?}", diags[0].message);
}

#[test]
fn vars_form_align_on_a_struct_field_is_refused_by_name() {
    // `@align(N)` on a `vars` field ADVANCES the allocation cursor. A struct
    // field never moves, so the spelling is refused with the one that asserts
    // rather than parsed as an offset expression beginning with an identifier.
    let src = "module m\nstruct Rec { head: u8, bridge: i16 @align(2) }\n";
    let (_file, diags) = parse_str(src);
    assert!(!diags.is_empty(), "expected a parse diagnostic, got none");
    assert!(
        diags.iter().any(|d| d.message
            == "`@align(N)` reserves space and belongs to a `vars` field; a struct field \
                asserts its alignment with `(align: N)`, which moves nothing"),
        "expected the spelling to be refused by name, got {diags:?}"
    );
}

#[test]
fn duplicate_at_offset_on_one_field_is_diagnosed() {
    // Two `@ offset` snapshots on one field cannot both be the author's claim.
    // The first is kept and the second refused, so the surviving assertion is the
    // one the reader sees first.
    let (_file, diags) = parse_str("module m\nstruct Rec { a: u8 @ 0 @ 1 }\n");
    assert!(
        diags.iter().any(|d| d.message == "duplicate `@ offset` on one field"),
        "expected the duplicate to be refused, got {diags:?}"
    );
}

#[test]
fn duplicate_field_align_on_one_field_is_diagnosed() {
    let (_file, diags) = parse_str("module m\nstruct Rec { a: u8 (align: 2) (align: 4) }\n");
    assert!(
        diags.iter().any(|d| d.message == "duplicate `(align:)` on one field"),
        "expected the duplicate to be refused, got {diags:?}"
    );
}

#[test]
fn an_unknown_field_attribute_keyword_names_the_one_that_belongs() {
    // `(align:)` is the only paren attribute a FIELD takes — `(size:)` is the
    // struct's. The refusal names the keyword that belongs here rather than
    // reporting a bare syntax error at the colon.
    let (_file, diags) = parse_str("module m\nstruct Rec { a: u8 (size: 2) }\n");
    assert!(
        diags.iter().any(|d| d.message == "expected `align:` in field attribute list"),
        "expected the field attribute list to name `align:`, got {diags:?}"
    );
}

#[test]
fn field_align_does_not_propagate_into_a_nested_structs_own_fields() {
    // SCOPE. A field of struct type asserts where the NESTED STRUCT STARTS; the
    // nested struct's own claims are judged in its own coordinates and nothing
    // reaches into them from the outer layout.
    //
    // Inner is internally clean: bridge@0 satisfies (align: 2). Placed in Outer
    // at offset 1, its bridge would sit at OUTER offset 1 — odd — so a check that
    // ever walked into a nested struct with the outer offset would report on
    // `bridge`. Nothing may.
    let src = "module m\n\
               struct Inner { bridge: u16 (align: 2), tail: u8 }\n\
               struct Outer { head: u8, nest: Inner (align: 1) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "Outer");
    assert_eq!(layout.expect("Outer should lay out").fields[1].offset, 1, "nest starts at 1");
    assert!(
        !diags.iter().any(|d| d.message.contains("bridge")),
        "a nested struct's own claim must not be re-judged in the outer struct: {diags:?}"
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    // The other half of the clause: the outer claim IS about where the nested
    // struct starts. `(align: 2)` on `nest` at offset 1 fails, and names `nest`.
    let src = "module m\n\
               struct Inner { bridge: u16 (align: 2), tail: u8 }\n\
               struct Outer { head: u8, nest: Inner (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Outer");
    assert_eq!(diags.len(), 1, "expected exactly the outer claim to fail, got {diags:?}");
    assert!(diags[0].message.contains("field nest"), "was {:?}", diags[0].message);
    assert!(diags[0].message.contains("but lands at offset 1"), "was {:?}", diags[0].message);
}

#[test]
fn field_align_catches_an_insertion_that_re_parities_a_bridge() {
    // The motivating shape: a hand-computed pad holding two word bridges even.
    // head@0 (2) + pad@2 (2) + bridge@4 — satisfied.
    let ok = "module m\nstruct Sc { head: u16, pad: u16 = 0, bridge: i16 (align: 2) }\n";
    let (file, diags) = parse_str(ok);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Sc");
    assert!(diags.is_empty(), "the padded shape must be silent: {diags:?}");

    // One byte inserted ABOVE the pad, which the pad's hand-computed width knows
    // nothing about: head@0 (2) + ins@2 (1) + pad@3 (2) + bridge@5. The claim is
    // on the bridge, not on the pad, so it survives the insertion and fires.
    let drifted =
        "module m\nstruct Sc { head: u16, ins: u8, pad: u16 = 0, bridge: i16 (align: 2) }\n";
    let (file, diags) = parse_str(drifted);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "Sc");
    let errs: Vec<&Diagnostic> = diags.iter().filter(|d| d.level == Level::Error).collect();
    assert_eq!(errs.len(), 1, "expected the bridge to fail, got {diags:?}");
    assert!(errs[0].message.contains("field bridge"), "was {:?}", errs[0].message);
    assert!(errs[0].message.contains("but lands at offset 5"), "was {:?}", errs[0].message);
}

// ---- struct pad markers `pad(N)` / `pad_to(N)` (§4.3.1) ------------------
//
// Anonymous byte runs between fields. `pad(N)` states a width; `pad_to(N)`
// states the offset the pad ENDS at and lets the compiler derive the width, so
// the number in the source is the one the author cares about rather than a
// count of the fields above it.

#[test]
fn pad_count_occupies_bytes_and_moves_the_next_field() {
    // a@0 (1 byte) + pad 3 → b@4, total 5.
    let src = "module m\nstruct S { a: u8, pad(3), b: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "a well-formed pad must be silent: {diags:?}");
    let layout = layout.expect("S should lay out");
    assert_eq!(layout.fields.len(), 2, "a pad is not a field: {:?}", layout.fields);
    assert_eq!(layout.fields[1].name, "b");
    assert_eq!(layout.fields[1].offset, 4);
    assert_eq!(layout.size, 5);
}

#[test]
fn pad_to_derives_the_width_that_lands_the_next_field_on_the_target() {
    // a@0 (1 byte), so the pad must derive width 4 - 1 = 3 to land b at 4.
    let src = "module m\nstruct S { a: u8, pad_to(4), b: u16 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "a satisfiable pad_to must be silent: {diags:?}");
    let layout = layout.expect("S should lay out");
    assert_eq!(layout.fields[1].offset, 4, "pad_to(4) must land b at 4");
    assert_eq!(layout.size, 6, "4 + sizeof(u16)");
}

#[test]
fn pad_to_at_the_cursor_is_legal_inert_and_silent() {
    // THE INERT ARM. a: u16 already reaches 2, so `pad_to(2)` has width 0. It is
    // still an assertion — it states that b begins at 2 — and making it an error
    // would fail the construct exactly when the layout is already correct.
    let src = "module m\nstruct S { a: u16, pad_to(2), b: u16 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "a zero-width pad_to must be silent: {diags:?}");
    let layout = layout.expect("S should lay out");
    assert_eq!(layout.fields[1].offset, 2);
    assert_eq!(layout.size, 4, "the inert pad must add nothing");
}

#[test]
fn pad_to_last_in_the_body_sets_the_struct_total_size() {
    // With no field after it, the pad's end offset IS the struct's size — the
    // total-size intent written in the same coordinate as every other pad target.
    // `(size: 8)` is an independent assertion and agrees, proving the two compose.
    let src = "module m\nstruct S (size: 8) { a: u16, pad_to(8) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "the tail pad must satisfy (size: 8) silently: {diags:?}");
    assert_eq!(layout.expect("S should lay out").size, 8);
}

#[test]
fn size_assertion_is_never_satisfied_by_auto_padding_the_tail() {
    // The counter-arm to the test above: WITHOUT a pad line, `(size: 8)` over a
    // 2-byte body still fails. `pad_to` supplies a declared pad's width, never a
    // pad's existence.
    let src = "module m\nstruct S (size: 8) { a: u16 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert_eq!(layout.expect("S should lay out").size, 2, "no pad was written, so none exists");
    assert_eq!(diags.len(), 1, "expected the size mismatch, got {diags:?}");
    assert!(diags[0].message.contains("declared size 8 but fields total 2"), "was {:?}", diags[0].message);
}

#[test]
fn pad_to_below_the_cursor_is_pad_overflow() {
    // a: u32 reaches 4; `pad_to(2)` is unsatisfiable — no width lands b at 2.
    let src = "module m\nstruct S { a: u32, pad_to(2), b: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    let d = &diags[0];
    assert_eq!(d.level, Level::Error, "pad-overflow is ERROR tier, got {:?}", d.level);
    assert!(d.message.contains("[layout.pad-overflow]"), "was {:?}", d.message);
    // Names the target, the following field, both offsets, and the delta.
    assert!(d.message.contains("struct S: pad_to(2) before field b"), "was {:?}", d.message);
    assert!(d.message.contains("already reach offset 4"), "was {:?}", d.message);
    assert!(d.message.contains("over by 2 byte(s)"), "was {:?}", d.message);
    // Names a remedy in the same coordinate, and refuses the hand-counted one.
    assert!(d.message.contains("Raise the target to 4, or remove 2 byte(s) above it"), "was {:?}", d.message);
    assert!(d.message.contains("Do not convert this to a hand-counted width"), "was {:?}", d.message);
}

#[test]
fn final_pad_to_below_the_cursor_names_the_end_of_the_struct() {
    // The final-pad variant: there is no following field to name.
    let src = "module m\nstruct S { a: u32, pad_to(2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("struct S: pad_to(2) at the end of the struct"),
        "was {:?}",
        diags[0].message
    );
    assert!(!diags[0].message.contains("before field"), "was {:?}", diags[0].message);
}

#[test]
fn pad_overflow_takes_width_zero_so_the_size_diff_still_prints() {
    // The recovery rule: the failed pad contributes 0 and layout CONTINUES, so a
    // `(size: N)` assertion on the same struct prints its own full diff instead
    // of being suppressed behind the pad error.
    let src = "module m\nstruct S (size: 99) { a: u32, pad_to(2), b: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    // Width 0: b sits immediately after a, and the total is 4 + 1.
    let layout = layout.expect("S should still lay out");
    assert_eq!(layout.fields[1].offset, 4, "the failed pad must take width 0");
    assert_eq!(layout.size, 5);
    assert_eq!(diags.len(), 2, "expected the pad error AND the size diff, got {diags:?}");
    assert!(diags.iter().any(|d| d.message.contains("[layout.pad-overflow]")), "got {diags:?}");
    let size_diag = diags
        .iter()
        .find(|d| d.message.contains("declared size 99"))
        .unwrap_or_else(|| panic!("no size diff in {diags:?}"));
    // The field-by-field diff, not just a headline.
    assert!(size_diag.message.contains("a @0"), "was {:?}", size_diag.message);
    assert!(size_diag.message.contains("b @4"), "was {:?}", size_diag.message);
    assert!(size_diag.message.contains("5 vs 99"), "was {:?}", size_diag.message);
}

#[test]
fn negative_pad_count_is_refused() {
    let src = "module m\nstruct S { a: u8, pad(-1), b: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    assert_eq!(diags[0].level, Level::Error);
    assert!(
        diags[0].message.contains("[layout.pad-count] struct S: pad(-1) — a pad count must be a non-negative comptime int"),
        "was {:?}",
        diags[0].message
    );
    // Recovered as width 0, so the rest of the struct still lays out.
    assert_eq!(layout.expect("S should still lay out").fields[1].offset, 1);
}

#[test]
fn negative_pad_to_target_is_refused_and_says_target_not_count() {
    // Same rule, different noun: `pad_to` takes a TARGET offset, and the message
    // must call the operand what the author wrote.
    let src = "module m\nstruct S { a: u8, pad_to(-1), b: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("[layout.pad-count] struct S: pad_to(-1) — a pad target must be a non-negative comptime int"),
        "was {:?}",
        diags[0].message
    );
}

#[test]
fn non_int_pad_operand_is_refused_by_the_pad_count_rule() {
    let src = "module m\nstruct S { a: u8, pad(\"three\"), b: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("[layout.pad-count]") && diags[0].message.contains("a pad count must be a non-negative comptime int"),
        "was {:?}",
        diags[0].message
    );
}

#[test]
fn pad_operands_are_comptime_expressions() {
    // As `@offset`, `(size:)` and `(align:)` all are — a named constant is as
    // good as a literal, in both spellings.
    let src = "module m\nconst GAP = 3\nconst TOP = 8\n\
               struct S { a: u8, pad(GAP), b: u8, pad_to(TOP), c: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let layout = layout.expect("S should lay out");
    assert_eq!(layout.fields[1].offset, 4, "a@0 + pad(3) → b@4");
    assert_eq!(layout.fields[2].offset, 8, "b ends at 5, pad_to(8) → c@8");
    assert_eq!(layout.size, 9);
}

#[test]
fn several_pads_between_the_same_two_fields_all_place() {
    // Each marker is placed once, in source order, and their widths accumulate.
    let src = "module m\nstruct S { a: u8, pad(2), pad_to(6), b: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    // a@0 → cursor 1; pad(2) → 3; pad_to(6) → 6.
    assert_eq!(layout.expect("S should lay out").fields[1].offset, 6);
}

#[test]
fn a_leading_pad_places_before_the_first_field() {
    let src = "module m\nstruct S { pad(2), a: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let layout = layout.expect("S should lay out");
    assert_eq!(layout.fields[0].offset, 2);
    assert_eq!(layout.size, 3);
}

// ---- `[layout.pad-hand-counted]` ----------------------------------------

#[test]
fn hand_counted_pad_before_an_aligned_field_warns() {
    // THE POISON ARM. `pad(1)` exists only to make `bridge` even; its width was
    // counted off `head` and goes stale the moment anything above changes.
    let src = "module m\nstruct S { head: u8, pad(1), bridge: i16 (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    // The lint changes nothing about the layout, and the claim it warns about
    // is satisfied — this fires on a CORRECT struct, which is the point.
    assert_eq!(layout.expect("S should lay out").fields[1].offset, 2);
    assert_eq!(diags.len(), 1, "expected exactly the lint, got {diags:?}");
    let d = &diags[0];
    assert_eq!(d.level, Level::Warning, "pad-hand-counted is a WARNING, got {:?}", d.level);
    assert!(d.message.contains("[layout.pad-hand-counted]"), "was {:?}", d.message);
    assert!(
        d.message.contains("struct S: pad(1) is followed by field bridge, which declares (align: 2)"),
        "was {:?}",
        d.message
    );
    assert!(d.message.contains("goes stale when any of them changes"), "was {:?}", d.message);
    // The fix-it names the exact replacement line, derived from the layout.
    assert!(d.message.contains("Write pad_to(2) instead"), "was {:?}", d.message);
}

#[test]
fn pad_to_before_an_aligned_field_does_not_warn() {
    // THE CONTROL ARM. The same struct written with the derived spelling is the
    // fix the lint asks for, so it must be silent — a lint that fired on both
    // would be telling the author to make no difference.
    let src = "module m\nstruct S { head: u8, pad_to(2), bridge: i16 (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "the derived spelling is the fix: {diags:?}");
}

#[test]
fn a_fixed_pad_before_an_unclaimed_field_does_not_warn() {
    // The signature is the PAIRING. A reserved-bytes run whose neighbour makes no
    // alignment claim is not evidence of a hand-counted width.
    let src = "module m\nstruct S { head: u8, pad(1), tail: u8 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn a_trailing_fixed_pad_has_no_following_field_to_pair_with() {
    let src = "module m\nstruct S { head: u16, bridge: i16 (align: 2), pad(3) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(layout.expect("S should lay out").size, 7);
}

#[test]
fn hand_counted_pad_lint_needs_the_aligned_field_to_be_the_next_one() {
    // "Immediately followed by" means no marker intervenes: with a `pad_to` in
    // between, the `pad(1)` is not the field's neighbour and nothing warns.
    let src = "module m\nstruct S { head: u8, pad(1), pad_to(2), bridge: i16 (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "only the neighbouring pad can pair: {diags:?}");
}

#[test]
fn hand_counted_pad_lint_is_silenced_by_its_allow() {
    // The honest case: a genuine reserved-bytes run that happens to precede an
    // aligned field.
    let src = "module m\n@allow(\"layout.pad-hand-counted\")\n\
               struct S { head: u8, pad(1), bridge: i16 (align: 2) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "the allow must silence the lint: {diags:?}");
}

#[test]
fn hand_counted_pad_lint_withdraws_from_a_refused_alignment_claim() {
    // A field whose `(align:)` is itself refused is already failing at the error
    // tier the author asked for. The heuristic warning has nothing to add, and it
    // would be false advice: its fix-it promises "the assertion still proves it",
    // and a refused assertion proves nothing.
    let src = "module m\nstruct S { head: u8, pad(1), bridge: i16 (align: 3) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert_eq!(diags.len(), 1, "expected exactly the alignment error, got {diags:?}");
    assert!(diags[0].message.contains("declares (align: 3)"), "was {:?}", diags[0].message);
    assert!(!diags[0].message.contains("pad-hand-counted"), "was {:?}", diags[0].message);
}

#[test]
fn hand_counted_pad_lint_does_not_double_report_an_unevaluatable_alignment() {
    // The lint evaluates the neighbouring `(align:)` expression only to QUOTE it,
    // and `check_struct_field_align` evaluates the same expression for the
    // verdict. A diagnostic raised by the quoting pass would be the verdict's own
    // diagnostic, said twice — so the quoting pass withdraws instead.
    let src = "module m\nstruct S { head: u8, pad(1), bridge: i16 (align: NOPE) }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert_eq!(diags.len(), 1, "the unknown name must be reported once, got {diags:?}");
    assert!(diags[0].message.contains("NOPE"), "was {:?}", diags[0].message);
}

// ---- pads vs the other §4.3 mechanisms -----------------------------------

#[test]
fn pad_bytes_are_exempt_from_the_odd_field_lint() {
    // A pad is a byte run with no access width, so its size makes no parity
    // claim. The control arm is the idiom it replaces: a 2-byte pad spelled
    // `u16` trips the lint purely because of the type its width was borrowed
    // from — the spurious subject this exemption retires.
    let padded = "module m\nstruct S { a: u8, pad(2), b: u8 }\n";
    let (file, diags) = parse_str(padded);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "a pad must never trip odd-field: {diags:?}");

    let named = "module m\nstruct S { a: u8, spare: u16, b: u8 }\n";
    let (file, diags) = parse_str(named);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert_eq!(diags.len(), 1, "the named-field spelling still warns: {diags:?}");
    assert!(diags[0].message.contains("[layout.odd-field]"), "was {:?}", diags[0].message);
    assert!(diags[0].message.contains("field spare"), "was {:?}", diags[0].message);
}

#[test]
fn a_pad_does_not_shift_the_odd_field_lint_off_a_real_field() {
    // The exemption is about the pad's OWN size, not about the fields after it:
    // a word pushed odd BY a pad still warns.
    let src = "module m\nstruct S { a: u16, pad(1), w: u16 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_struct(&file, "S");
    assert_eq!(diags.len(), 1, "expected the odd-field warning, got {diags:?}");
    assert!(diags[0].message.contains("field w"), "was {:?}", diags[0].message);
    assert!(diags[0].message.contains("odd offset 3"), "was {:?}", diags[0].message);
}

#[test]
fn sizeof_counts_a_pads_bytes() {
    let src = "module m\nstruct S { a: u8, pad(3), b: u8 }\nconst N = sizeof(S)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(int(5)), "1 + 3 + 1");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn offsetof_cannot_name_a_pad() {
    // A pad is not a field you read, so the language must make reading it
    // inexpressible: there is no name to pass, and the closest spelling resolves
    // to nothing.
    let src = "module m\nstruct S { a: u8, pad(3), b: u8 }\nconst N = offsetof(S, pad)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("offsetof") && diags[0].message.contains("pad"),
        "was {:?}",
        diags[0].message
    );
}

#[test]
fn a_field_named_pad_coexists_with_pad_markers() {
    // `pad` is a marker only when immediately followed by `(`; as a field name it
    // is an ordinary identifier, and `offsetof` reaches THAT one.
    let src = "module m\nstruct S { pad: u8, pad(3), b: u8 }\n\
               const P = offsetof(S, pad)\nconst B = offsetof(S, b)\n";
    let (v, diags) = eval(src, "P");
    assert_eq!(v, Some(int(0)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let (v, diags) = eval(src, "B");
    assert_eq!(v, Some(int(4)), "the named field, then the anonymous 3-byte run");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn a_pad_inside_a_nested_struct_counts_toward_the_outer_layout() {
    // Every consumer of a pad's width agrees, including the one that reaches it
    // through `size_of_ty` rather than the pad walk itself.
    // `lead: u16` keeps the 4-byte nested struct at an even offset, so this test
    // is about the pad's contribution and not the odd-field lint.
    let src = "module m\nstruct Inner { a: u8, pad(3) }\n\
               struct Outer { lead: u16, nest: Inner, tail: u8 }\n\
               const N = offsetof(Outer, tail)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(int(6)), "2 + sizeof(Inner) = 2 + 4");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}
