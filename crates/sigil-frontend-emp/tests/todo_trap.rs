//! `todo!` / `unreachable!` statement traps (S2-D11(e), ratified IN at the
//! v1 freeze): statement-position holes that assemble to the 68k ILLEGAL
//! opcode ($4AFC — a guaranteed trap) so WIP files build and RUN to the hole.
//! `todo!` names itself at build time (`[todo.present]`, warning tier, one
//! per site, carrying the optional message); `unreachable!` is the silent,
//! intentional sibling. 68k-only in v1 (`[todo.non-68k]`), the script/offsets
//! precedent.
//!
//! Helpers mirror `tests/script.rs` (same lowering entry, same single-section
//! link harness).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

/// Lower `src` (asserting a clean parse) and return the module + FULL
/// diagnostics (this suite asserts on levels, not just messages).
fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] })
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

// ---- 1. lowering: the trap bytes -------------------------------------------

#[test]
fn todo_lowers_to_illegal_opcode() {
    let src = "\
module m
proc p () {
    nop
    todo!
    nop
}
";
    let (m, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| matches!(d.level, Level::Error)),
        "no errors expected: {diags:?}"
    );
    // nop = 4E71, illegal = 4AFC, nop = 4E71
    assert_eq!(linked_bytes(&m), vec![0x4E, 0x71, 0x4A, 0xFC, 0x4E, 0x71]);
}

#[test]
fn unreachable_lowers_to_the_same_trap_silently() {
    let src = "\
module m
proc p () {
    unreachable!
}
";
    let (m, diags) = lower(src);
    assert_eq!(linked_bytes(&m), vec![0x4A, 0xFC]);
    assert!(
        !diags.iter().any(|d| d.message.contains("[todo.present]")),
        "unreachable! must not report [todo.present]: {diags:?}"
    );
}

// ---- 2. the [todo.present] diagnostic --------------------------------------

#[test]
fn todo_reports_present_diagnostic_with_message() {
    let src = "\
module m
proc p () {
    todo!(\"wire the seed spawn\")
}
";
    let (m, diags) = lower(src);
    let d = diags
        .iter()
        .find(|d| d.message.contains("[todo.present]"))
        .unwrap_or_else(|| panic!("expected a [todo.present] diagnostic, got {diags:?}"));
    assert!(matches!(d.level, Level::Warning), "warning tier (build still succeeds): {d:?}");
    assert!(
        d.message.contains("wire the seed spawn"),
        "the site's message rides the diagnostic: {}",
        d.message
    );
    // The message form still emits the trap bytes.
    assert_eq!(linked_bytes(&m), vec![0x4A, 0xFC]);
}

#[test]
fn bare_todo_reports_present_without_message() {
    let src = "\
module m
proc p () {
    todo!
}
";
    let (_, diags) = lower(src);
    assert!(
        diags.iter().any(|d| d.message.contains("[todo.present]") && matches!(d.level, Level::Warning)),
        "expected warning-tier [todo.present]: {diags:?}"
    );
}

#[test]
fn each_todo_site_reports_separately() {
    let src = "\
module m
proc p () {
    todo!(\"first hole\")
    todo!(\"second hole\")
}
";
    let (_, diags) = lower(src);
    let count = diags.iter().filter(|d| d.message.contains("[todo.present]")).count();
    assert_eq!(count, 2, "one [todo.present] per site: {diags:?}");
}

#[test]
fn empty_paren_todo_is_the_bare_form() {
    // `todo!()` — the Rust-muscle-memory spelling — parses like bare `todo!`.
    let src = "\
module m
proc p () {
    todo!()
}
";
    let (m, diags) = lower(src);
    assert_eq!(linked_bytes(&m), vec![0x4A, 0xFC]);
    assert!(
        diags.iter().any(|d| d.message.contains("[todo.present]")),
        "still reports the hole: {diags:?}"
    );
}

#[test]
fn todo_before_closing_brace_on_same_line() {
    // The last-statement-before-`}` shape parses like `{ nop }` does.
    let src = "\
module m
proc p () { todo! }
";
    let (m, diags) = lower(src);
    assert_eq!(linked_bytes(&m), vec![0x4A, 0xFC]);
    assert!(
        diags.iter().any(|d| d.message.contains("[todo.present]")),
        "still reports the hole: {diags:?}"
    );
}

// ---- 3. 68k-only (v1) -------------------------------------------------------

#[test]
fn todo_in_z80_section_is_non_68k_error() {
    let src = "\
module m
section s (cpu: z80, vma: $8000) {
    proc p () {
        todo!
    }
}
";
    let (_, diags) = lower(src);
    assert!(
        diags
            .iter()
            .any(|d| d.message.contains("[todo.non-68k]") && matches!(d.level, Level::Error)),
        "expected [todo.non-68k] error: {diags:?}"
    );
}

// ---- 4. script bodies share the statement grammar ---------------------------

#[test]
fn todo_inside_script_body_parses_and_lowers() {
    let src = "\
module m
newtype ScriptPc = u16
struct S (size: $24) {
    _pad0: [u8; $20],
    resume: ScriptPc @ $20,
    _pad1: [u8; 2] @ $22,
}
script brain (a0: *S) (encoding: word_offsets) shows done {
    todo!(\"the windup\")
    yield
}
proc done () { rts }
";
    let (_, diags) = lower(src);
    assert!(
        !diags.iter().any(|d| matches!(d.level, Level::Error)),
        "script body accepts todo!: {diags:?}"
    );
    assert!(
        diags.iter().any(|d| d.message.contains("[todo.present]") && d.message.contains("the windup")),
        "expected [todo.present] from the script body: {diags:?}"
    );
}
