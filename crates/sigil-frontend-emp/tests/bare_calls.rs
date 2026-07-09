//! Spec 2, Plan 7 (pitcher_plant tranche) — U1 / D-PP.1 + D-PP.2: bare
//! directive-style statement calls and registers as comptime call arguments.
//!
//! D-PP.1 — in proc-body statement position, a leading bareword that is NOT a
//! recognized mnemonic for the section's CPU (and not `jbra`/`jbsr`) and RESOLVES
//! to an in-scope comptime fn parses as a call: `name` (zero args) or
//! `name arg, arg, …` (comma-separated comptime expressions). Pure sugar for the
//! already-working paren form — both spellings legal and byte-identical.
//!
//! D-PP.2 — a new comptime-only type name `Reg` usable in comptime fn signatures;
//! in comptime CALL-ARGUMENT position (both bare and paren spellings) a bareword
//! naming a machine register of the current CPU parses as a register literal
//! (`Value::Reg`), which `{r}` splices back into a template's operand position.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

// ---- shared helpers (mirror lower_corpus.rs) --------------------------------

/// Parse + lower `src` for the 68k, asserting a clean parse. Returns the module
/// and lowering diagnostics.
fn lower(src: &str) -> (sigil_ir::Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] })
}

fn section<'a>(module: &'a sigil_ir::Module, name: &str) -> &'a Section {
    module.sections.iter().find(|s| s.name == name).unwrap_or_else(|| {
        panic!("no section `{name}`")
    })
}

fn label_offset(sec: &Section, name: &str) -> u32 {
    sec.labels.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("no label `{name}`")).offset
}

fn linked_section_bytes(module: &sigil_ir::Module, name: &str) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

fn errors(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.as_str()).collect()
}

/// The bytes of proc `name`'s body (from its label to the section end / next
/// proc), for byte-identical spelling comparisons.
fn proc_bytes(module: &sigil_ir::Module, name: &str, len: usize) -> Vec<u8> {
    let text = section(module, "text");
    let off = label_offset(text, name) as usize;
    let linked = linked_section_bytes(module, "text");
    linked[off..off + len].to_vec()
}

// =============================================================================
// D-PP.1 — bare statement calls
// =============================================================================

/// A one-arg bare call `set_timer 64` splices its Code exactly like the paren
/// form `set_timer(64)`.
const SET_TIMER: &str = "\
module m

comptime fn set_timer(v: u8) -> Code {
    return asm {
        move.b #{v}, d0
    }
}

proc bare (a0: *u8) {
    set_timer 64
    rts
}
proc paren (a0: *u8) {
    set_timer(64)
    rts
}
";

#[test]
fn bare_one_arg_call_matches_paren_bytes() {
    let (module, diags) = lower(SET_TIMER);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // `move.b #64, d0` = 10 3C 00 40, then rts = 4E 75 → 6 bytes each.
    let bare = proc_bytes(&module, "bare", 6);
    let paren = proc_bytes(&module, "paren", 6);
    assert_eq!(bare, vec![0x10, 0x3C, 0x00, 0x40, 0x4E, 0x75], "bare-call body bytes");
    assert_eq!(bare, paren, "bare and paren spellings must be byte-identical");
}

/// A zero-arg bare call `nop_twice` (no parens, no args).
const ZERO_ARG: &str = "\
module m

comptime fn nop_twice() -> Code {
    return asm {
        nop
        nop
    }
}

proc bare (a0: *u8) {
    nop_twice
    rts
}
";

#[test]
fn bare_zero_arg_call() {
    let (module, diags) = lower(ZERO_ARG);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // nop = 4E 71 twice, then rts = 4E 75.
    let bytes = proc_bytes(&module, "bare", 6);
    assert_eq!(bytes, vec![0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x75]);
}

/// A multi-arg bare call with expression arguments: `set_two 1+2, $FF`.
const MULTI_ARG: &str = "\
module m

comptime fn set_two(a: u8, b: u8) -> Code {
    return asm {
        moveq #{a}, d0
        move.b #{b}, d1
    }
}

proc bare (a0: *u8) {
    set_two 1+2, $FF
    rts
}
";

#[test]
fn bare_multi_arg_call_with_expressions() {
    let (module, diags) = lower(MULTI_ARG);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // moveq #3, d0 = 70 03; move.b #$FF, d1 = 12 3C 00 FF; rts = 4E 75.
    let bytes = proc_bytes(&module, "bare", 8);
    assert_eq!(bytes, vec![0x70, 0x03, 0x12, 0x3C, 0x00, 0xFF, 0x4E, 0x75]);
}

// =============================================================================
// D-PP.2 — registers as comptime call arguments
// =============================================================================

/// A Reg param spliced into a template's operand position: `neg.w {r}` with
/// r = d3. `neg.w d3` = 44 43.
const FACING_ABS: &str = "\
module m

comptime fn facing_abs(r: Reg) -> Code {
    return asm {
        neg.w {r}
    }
}

proc bare (a0: *u8) {
    facing_abs d3
    rts
}
proc paren (a0: *u8) {
    facing_abs(d3)
    rts
}
";

#[test]
fn reg_param_bare_and_paren_splice() {
    let (module, diags) = lower(FACING_ABS);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // neg.w d3 = 44 43, rts = 4E 75.
    let bare = proc_bytes(&module, "bare", 4);
    let paren = proc_bytes(&module, "paren", 4);
    assert_eq!(bare, vec![0x44, 0x43, 0x4E, 0x75], "neg.w d3 + rts");
    assert_eq!(bare, paren, "Reg arg works in both spellings, identical bytes");
}

/// A nested call as an argument: `outer inner(2), d0` — `inner(2)` is a paren
/// call whose Code arg... no: `inner(2)` here returns an int the outer splices.
const NESTED_CALL: &str = "\
module m

comptime fn inner(n: u8) -> u8 {
    return n + 1
}
comptime fn outer(v: u8, r: Reg) -> Code {
    return asm {
        move.b #{v}, {r}
    }
}

proc bare (a0: *u8) {
    outer inner(2), d0
    rts
}
";

#[test]
fn bare_call_with_nested_call_and_reg_args() {
    let (module, diags) = lower(NESTED_CALL);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // inner(2) = 3 → move.b #3, d0 = 10 3C 00 03, rts = 4E 75.
    let bytes = proc_bytes(&module, "bare", 6);
    assert_eq!(bytes, vec![0x10, 0x3C, 0x00, 0x03, 0x4E, 0x75]);
}

// =============================================================================
// Error paths
// =============================================================================

/// A bareword that resolves to nothing keeps today's "not a recognized 68000
/// mnemonic" error EXACTLY.
#[test]
fn unknown_bareword_keeps_mnemonic_error() {
    let src = "\
module m
proc p (a0: *u8) {
    frobnicate d0
    rts
}
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("`frobnicate` is not a recognized 68000 mnemonic")),
        "unknown bareword must keep the mnemonic error, got: {errs:?}"
    );
}

/// A bareword resolving to a NON-fn comptime value (a const) in statement
/// position: a specific error naming what it is.
#[test]
fn bareword_naming_const_is_specific_error() {
    let src = "\
module m
const speed: u8 = 5
proc p (a0: *u8) {
    speed
    rts
}
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("`speed` names a const, not a comptime fn")),
        "const in statement position must name the kind, got: {errs:?}"
    );
}

/// Same for an enum type name — and the article must agree ("an enum", never
/// "a enum").
#[test]
fn bareword_naming_enum_is_specific_error_with_correct_article() {
    let src = "\
module m
enum Ani: u8 { Idle = 0, Shoot = 1 }
proc p (a0: *u8) {
    Ani
    rts
}
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("`Ani` names an enum, not a comptime fn")),
        "enum in statement position must name the kind with a grammatical article, got: {errs:?}"
    );
}

/// A call to a comptime fn that does NOT return Code: statement position
/// requires Code.
#[test]
fn non_code_fn_in_statement_position_errors() {
    let src = "\
module m
comptime fn compute(v: u8) -> u8 { return v + 1 }
proc p (a0: *u8) {
    compute 3
    rts
}
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.to_lowercase().contains("code")),
        "non-Code fn at statement position must require Code, got: {errs:?}"
    );
}

/// A register name into a NON-Reg param (a u8 param): a type error naming the
/// expected type.
#[test]
fn register_into_non_reg_param_is_type_error() {
    let src = "\
module m
comptime fn set_timer(v: u8) -> Code {
    return asm { move.b #{v}, d0 }
}
proc p (a0: *u8) {
    set_timer d3
    rts
}
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("register is not a valid `u8` argument")),
        "register into a u8 param must name the expected type, got: {errs:?}"
    );
}

/// A Reg where the fn body tries to splice it as an immediate (a non-register
/// splice position): a typed error naming the operand class.
#[test]
fn reg_into_non_register_splice_is_typed_error() {
    let src = "\
module m
comptime fn bad(r: Reg) -> Code {
    return asm { move.b #{r}, d0 }
}
proc p (a0: *u8) {
    bad d3
    rts
}
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    // The Reg reaches an `#imm` position, which requires an integer — a typed
    // error naming the operand class (the value's type is `reg`).
    assert!(
        errs.iter().any(|e| e.contains("immediate must be an integer") && e.contains("reg")),
        "a Reg spliced into #imm position must be a typed error naming the class, got: {errs:?}"
    );
}

// =============================================================================
// Interaction — jbra must not be shadowed; fallthrough analysis unchanged
// =============================================================================

/// A comptime fn named `jbra` must NOT shadow the T2 jbra mnemonic: `jbra`
/// still lowers as the auto-reaching branch.
#[test]
fn comptime_fn_named_jbra_does_not_shadow_mnemonic() {
    let src = "\
module m
comptime fn jbra() -> Code { return asm { nop } }
proc p (a0: *u8) {
    jbra .done
.done:
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // jbra to an adjacent label relaxes to bra.s .done = 60 00 (offset 0), then
    // rts = 4E 75. The key point: `jbra` lowered as a BRANCH (60..), not as a
    // nop (4E 71) from the shadowing comptime fn.
    let bytes = proc_bytes(&module, "p", 2);
    assert_eq!(bytes[0], 0x60, "jbra stays the branch mnemonic, not the shadow fn");
}

/// A bare call whose instantiated Code ends in `rts` satisfies proc termination
/// exactly as the paren form does (the fallthrough analysis inspects the last
/// emitted mnemonic, which is `rts` from the splice).
#[test]
fn bare_call_ending_in_rts_matches_paren_fallthrough() {
    let src = "\
module m
comptime fn ret() -> Code { return asm { rts } }
proc bare (a0: *u8) {
    ret
}
proc paren (a0: *u8) {
    ret()
}
";
    let (_module, diags) = lower(src);
    // Neither should warn about undeclared fallthrough (both end in a spliced rts).
    let fall: Vec<&str> = diags
        .iter()
        .filter(|d| d.message.contains("[proc.undeclared-fallthrough]"))
        .map(|d| d.message.as_str())
        .collect();
    assert!(fall.is_empty(), "bare and paren both terminate via spliced rts: {fall:?}");
}

// =============================================================================
// D-PP.4 — named call arguments: the bare-spelling decision
// =============================================================================
//
// D-PP.4 is otherwise a paren-form-only feature (see `eval_fns.rs`'s
// "D-PP.4" section for the binder tests). The bare statement-call spelling
// (D-PP.1) reverses each parsed instruction OPERAND back into a positional
// `Arg` (`operand_to_arg` in `eval/asm.rs`) — there is no operand shape for
// `name: expr`, because the operand grammar's trailing-size/local-label
// machinery already claims a bare `ident :` prefix in ways that would be
// genuinely ambiguous to repurpose (`.draw:` label syntax, and — decisively —
// the operand parser already stops at the colon with a diagnostic today, so
// this is confirmed a PARSE-TIME rejection, not a semantic gap to plumb).
// Fallback taken: named args are PAREN-FORM ONLY; a bare-form `k: v` stays a
// loud parse error (never silently dropped or silently repositioned). The
// tranche's only named-arg call (`spawn(SeedDef, offset: ..., flip: ...)`) is
// already paren-form in the exhibit, so this fallback satisfies the
// acceptance corpus without bare-form grammar surgery.

/// `f offset: 1` (bare statement call, named-looking arg) must NOT parse
/// clean and must NOT silently reinterpret `offset` as a positional register/
/// path arg while dropping `: 1` — it stays the pre-existing loud parse error
/// at the colon.
#[test]
fn bare_form_named_looking_arg_is_a_loud_parse_error() {
    let src = "\
module m

comptime fn f(offset: int) -> int { return offset }

proc p (a0: *u8) {
    f offset: 1
    rts
}
";
    let (_file, diags) = sigil_frontend_emp::parse_str(src);
    assert!(
        !diags.is_empty(),
        "bare-form `name: expr` must be a loud parse error, not a silent parse"
    );
    assert!(
        diags.iter().any(|d| d.level == Level::Error),
        "expected at least one ERROR diagnostic, got: {diags:?}"
    );
}
