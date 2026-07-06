//! Integration tests for `offsets Name { Variant: target, ... }` — the REVERSE
//! direction (Spec 2, Plan 7 backlog #3, Task 5): the comptime ordinal
//! constants `Name.Variant` (0-based index of the member) and `Name.count`
//! (member count). These are plain comptime ints (`Value::Int`), not a
//! distinct enum-like type. Forward emission (`dc.w target - Name`) is a
//! separate, later task (Task 6) and is NOT exercised here — the `target`
//! expr of each member is never evaluated by this task, so it can name any
//! identifier (it need not resolve to a real const/label).
//!
//! Mirrors the harness in `eval_consts.rs`: parse a full `.emp` file (asserting
//! a clean parse), then evaluate a named const that references the offsets
//! table via [`eval_const`], asserting on the resulting [`Value`] and
//! diagnostics.
use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;
use sigil_ir::backend::Cpu;

/// Parse `src` (asserting a clean parse) and evaluate the const named `name`.
fn eval(src: &str, name: &str) -> (Option<Value>, Vec<sigil_span::Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_const(&file, name)
}

fn int(n: i128) -> Value {
    Value::Int(n)
}

#[test]
fn ordinals_are_zero_based_in_declaration_order() {
    let src = "module m\noffsets M { A: t0, B: t1, C: t2 }\nconst X = M.A\n";
    let (v, diags) = eval(src, "X");
    assert_eq!(v, Some(int(0)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    let src = "module m\noffsets M { A: t0, B: t1, C: t2 }\nconst X = M.B\n";
    let (v, diags) = eval(src, "X");
    assert_eq!(v, Some(int(1)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    let src = "module m\noffsets M { A: t0, B: t1, C: t2 }\nconst X = M.C\n";
    let (v, diags) = eval(src, "X");
    assert_eq!(v, Some(int(2)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn count_is_member_count() {
    let src = "module m\noffsets M { A: t0, B: t1, C: t2 }\nconst X = M.count\n";
    let (v, diags) = eval(src, "X");
    assert_eq!(v, Some(int(3)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn unknown_member_is_an_error() {
    let src = "module m\noffsets M { A: t0, B: t1 }\nconst X = M.Nope\n";
    let (v, diags) = eval(src, "X");
    assert_eq!(v, Some(Value::Poison));
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(diags[0].message.contains("no member"), "diagnostic was {:?}", diags[0].message);
}

#[test]
fn duplicate_member_name_is_an_error() {
    // A duplicate member name is a hard error (its ordinal would be ambiguous).
    // The check is a once-per-compile lowering pass (`lower::validate_offsets`),
    // so it is observed through `lower_module`, not the single-evaluator
    // `eval_const` path. Independent of any reference to `M`.
    let src = "module m\noffsets M { A: t0, A: t1 }\n";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.is_empty(), "expected a clean parse, got {pdiags:?}");
    let (_module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(diags[0].message.contains("duplicate"), "diagnostic was {:?}", diags[0].message);
}

#[test]
fn duplicate_member_reported_once_through_full_lowering() {
    // Regression: the duplicate-member check must fire ONCE per compile, not
    // once per per-item evaluator. `lower_module` builds a fresh evaluator per
    // `data` item (for `Name.Variant` resolution), so a naive check in the
    // evaluator's `index_items` would emit N copies for N data items. With ≥2
    // data items present, assert EXACTLY ONE "duplicate" diagnostic.
    let src = "\
module m
offsets M { A: t0, A: t1 }
data D1: [u8; 1] = [1]
data D2: [u8; 1] = [2]
data D3: [u8; 1] = [3]
";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.is_empty(), "expected a clean parse, got {pdiags:?}");
    let (_module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    let dups: Vec<_> = diags.iter().filter(|d| d.message.contains("duplicate")).collect();
    assert_eq!(dups.len(), 1, "expected exactly one duplicate diagnostic, got {diags:?}");
}

#[test]
fn ordinal_used_arithmetically() {
    // Ordinals are plain comptime ints usable like any other (D-P2 taste): no
    // distinct enum-like wrapper, no coercion needed.
    let src = "module m\noffsets M { A: t0, B: t1, C: t2 }\nconst X = M.B + 10\n";
    let (v, diags) = eval(src, "X");
    assert_eq!(v, Some(int(11)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}
