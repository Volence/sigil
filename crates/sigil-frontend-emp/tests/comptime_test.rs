//! `comptime test` blocks (S2-D11(a), ratified IN at the freeze): colocated
//! comptime tests, stripped from emission ALWAYS, run by `sigil test`. The
//! `expect_error` variant is the "this must NOT compile" test (absorbing
//! research T3-g `EXPECT`). v1 scope: module-local (tests exercise the
//! module's own comptime fns — the colocated case; cross-module imports are
//! the recorded next increment).

use sigil_frontend_emp::eval::run_module_tests;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

fn lower(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    (module, diags.into_iter().map(|d| d.message).collect())
}

fn linked_bytes(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

fn run(src: &str) -> Vec<sigil_frontend_emp::eval::TestOutcome> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    run_module_tests(&file, None, &[])
}

// ---- 1. stripped from emission --------------------------------------------------

#[test]
fn test_blocks_emit_nothing() {
    let with = "\
module m
comptime fn double(x: int) -> int {
    return x * 2
}
comptime test \"double doubles\" {
    ensure(double(2) == 4, \"2*2\")
}
data D: [u8; 1] = [double(3)]
";
    let without = "\
module m
comptime fn double(x: int) -> int {
    return x * 2
}
data D: [u8; 1] = [double(3)]
";
    let (m1, d1) = lower(with);
    assert!(d1.is_empty(), "clean lower: {d1:?}");
    let (m2, d2) = lower(without);
    assert!(d2.is_empty(), "{d2:?}");
    assert_eq!(linked_bytes(&m1), linked_bytes(&m2), "a test block is byte-free");
}

// ---- 2. the runner ---------------------------------------------------------------

#[test]
fn passing_test_reports_ok() {
    let src = "\
module m
comptime fn double(x: int) -> int {
    return x * 2
}
comptime test \"double doubles\" {
    ensure(double(21) == 42, \"the answer\")
}
";
    let results = run(src);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed, "diags: {:?}", results[0].diags);
    assert_eq!(results[0].name, "double doubles");
}

#[test]
fn failing_ensure_reports_failed_with_the_message() {
    let src = "\
module m
comptime test \"arithmetic is broken\" {
    ensure(1 + 1 == 3, \"one plus one is {1 + 1}\")
}
";
    let results = run(src);
    assert_eq!(results.len(), 1);
    assert!(!results[0].passed);
    assert!(
        results[0].diags.iter().any(|d| d.message.contains("one plus one is 2")),
        "the guard's interpolated message rides the failure: {:?}",
        results[0].diags
    );
}

#[test]
fn expect_error_passes_on_a_matching_diagnostic() {
    let src = "\
module m
struct S { a: u8 }
comptime test \"missing field must not compile\" (expect_error: \"[struct.missing-field]\") {
    let bad = S{ }
}
";
    let results = run(src);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed, "diags: {:?}", results[0].diags);
    assert!(results[0].diags.is_empty(), "a passing expect_error swallows the captured diags");
}

#[test]
fn expect_error_fails_on_a_clean_body() {
    let src = "\
module m
comptime test \"expected an error\" (expect_error: \"[struct.missing-field]\") {
    let fine = 1 + 1
}
";
    let results = run(src);
    assert_eq!(results.len(), 1);
    assert!(!results[0].passed);
    assert!(
        results[0].diags.iter().any(|d| d.message.contains("[struct.missing-field]")),
        "the failure names the diagnostic it expected: {:?}",
        results[0].diags
    );
}

// ---- 3. declaration checks --------------------------------------------------------

#[test]
fn duplicate_test_names_error_at_lower() {
    let src = "\
module m
comptime test \"same\" {
    ensure(true, \"a\")
}
comptime test \"same\" {
    ensure(true, \"b\")
}
";
    let (_, msgs) = lower(src);
    assert!(
        msgs.iter().any(|m| m.contains("same") && m.contains("twice")),
        "duplicate test names are refused: {msgs:?}"
    );
}

#[test]
fn pub_comptime_test_is_rejected() {
    let (_, perrs) = parse_str("module m\npub comptime test \"t\" {\n    ensure(true, \"x\")\n}\n");
    assert!(
        perrs.iter().any(|d| d.message.contains("`pub` is not valid")),
        "tests are not exportable: {perrs:?}"
    );
}

// ---- stage-2 pins (Item-10 review) ----------------------------------------------

#[test]
fn section_nested_test_is_rejected_loudly() {
    // A section-nested test would parse, strip, and silently never run —
    // the worst failure mode a test feature can have (review M1).
    let (_, perrs) = parse_str(
        "module m\nsection s (vma: $100) {\n    comptime test \"hidden\" {\n        ensure(false, \"never runs\")\n    }\n}\n",
    );
    assert!(
        perrs.iter().any(|d| d.message.contains("[test.in-section]")),
        "expected [test.in-section]: {perrs:?}"
    );
}

#[test]
fn expect_error_does_not_pass_on_a_warning() {
    // "Must not compile" means an ERROR — a warning containing the id
    // compiles fine (review M4). A struct with a u16 at an odd offset
    // produces the WARNING-tier [layout.odd-field] when constructed.
    let src = "\
module m
struct Odd { a: u8, b: u16 }
comptime test \"warning is not an error\" (expect_error: \"[layout.odd-field]\") {
    let v = Odd{ a: 1, b: 2 }
}
";
    let results = run(src);
    assert_eq!(results.len(), 1);
    assert!(!results[0].passed, "a warning must not satisfy expect_error: {results:?}");
}

#[test]
fn duplicate_names_fail_under_the_runner_too() {
    // The build path's validate pass never runs under `sigil test` (M3).
    let src = "\
module m
comptime test \"same\" {
    ensure(true, \"a\")
}
comptime test \"same\" {
    ensure(true, \"b\")
}
";
    let results = run(src);
    assert!(
        results.iter().any(|r| !r.passed && r.diags.iter().any(|d| d.message.contains("twice"))),
        "the runner refuses the duplicate: {results:?}"
    );
}
