//! `[layout.odd-item]` — D2.29's audit-amendment companion check: alignment
//! bytes are never INSERTED automatically, but a 68k `proc` (error — an odd
//! instruction address is a guaranteed address-error crash) or a word/long-
//! bearing data item (warning) landing at an odd FINAL address is diagnosed
//! with the machine-applicable "insert `align 2`" fix-it. Z80 sections and
//! `@as_compat` modules are exempt. Final addresses exist post-link (D2.25),
//! so the check rides the LinkAssert channel — these tests drive the same
//! `resolve_layout` → `check_link_asserts` path the CLI uses.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

/// Lower + link + decide the link asserts; returns every diagnostic from the
/// whole path (lowering + asserts).
fn full(src: &str) -> Vec<Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, mut diags): (Module, Vec<Diagnostic>) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    let resolved =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve_layout");
    diags.extend(sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &module.link_asserts));
    diags
}

fn odd_items(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| d.message.contains("[layout.odd-item]")).collect()
}

// ---- 1. procs: error ---------------------------------------------------------

#[test]
fn odd_proc_is_error_with_fixit() {
    let src = "\
module m
data D: [u8; 1] = [1]
proc p () {
    rts
}
";
    let diags = full(src);
    let odd = odd_items(&diags);
    assert_eq!(odd.len(), 1, "exactly the proc flagged: {diags:?}");
    assert!(matches!(odd[0].level, Level::Error), "odd proc = address-error crash = error");
    assert!(odd[0].message.contains("insert `align 2`"), "fix-it named: {}", odd[0].message);
    assert!(odd[0].message.contains("`p`"), "the item is named: {}", odd[0].message);
}

#[test]
fn even_proc_is_silent() {
    let src = "\
module m
data D: [u8; 2] = [1, 2]
proc p () {
    rts
}
";
    let diags = full(src);
    assert!(odd_items(&diags).is_empty(), "even proc is clean: {diags:?}");
}

#[test]
fn align_2_silences_the_odd_proc() {
    let src = "\
module m
data D: [u8; 1] = [1]
align 2
proc p () {
    rts
}
";
    let diags = full(src);
    assert!(odd_items(&diags).is_empty(), "the fix-it fixes it: {diags:?}");
}

// ---- 2. data: warning, only when word/long-bearing ----------------------------

#[test]
fn odd_word_bearing_data_is_warning() {
    let src = "\
module m
data D1: [u8; 1] = [1]
data D2: [u16; 2] = [$1111, $2222]
";
    let diags = full(src);
    let odd = odd_items(&diags);
    assert_eq!(odd.len(), 1, "exactly D2 flagged: {diags:?}");
    assert!(matches!(odd[0].level, Level::Warning), "wordy data = warning tier");
    assert!(odd[0].message.contains("`D2`"), "named: {}", odd[0].message);
}

#[test]
fn odd_pure_byte_data_is_silent() {
    let src = "\
module m
data D1: [u8; 1] = [1]
data D2: [u8; 3] = [2, 3, 4]
";
    let diags = full(src);
    assert!(odd_items(&diags).is_empty(), "byte-only data has no alignment need: {diags:?}");
}

#[test]
fn odd_offsets_table_is_warning() {
    // An offsets table is dc.w words by construction.
    let src = "\
module m
data Pad: [u8; 1] = [0]
data A: [u8; 2] = [1, 2]
offsets T {
    First: A,
}
";
    let diags = full(src);
    let odd = odd_items(&diags);
    assert!(
        odd.iter().any(|d| d.message.contains("`T`") && matches!(d.level, Level::Warning)),
        "the odd offsets table warns: {diags:?}"
    );
}

// ---- 3. exemptions -------------------------------------------------------------

#[test]
fn z80_sections_are_exempt() {
    let src = "\
module m
section z (cpu: z80, vma: $0000) {
    data D1: [u8; 1] = [1]
    proc p () {
        ret
    }
}
";
    let diags = full(src);
    assert!(odd_items(&diags).is_empty(), "Z80 has no alignment requirement: {diags:?}");
}

#[test]
fn allow_attr_opts_out_the_warning_tier_only() {
    // `@allow("layout.odd-item")` — the first real consumer of the parsed
    // `@allow` module attribute — silences the DATA warning (aeon's byte-read
    // dac descriptor stride), but never the Code tier: a guaranteed
    // address-error crash is not lint-allowable.
    let src = "\
module m
@allow(\"layout.odd-item\")
data D1: [u8; 1] = [1]
data D2: [u16; 1] = [$1111]
proc p () {
    rts
}
";
    let diags = full(src);
    let odd = odd_items(&diags);
    assert!(
        !odd.iter().any(|d| d.message.contains("`D2`")),
        "the data warning is allowed away: {diags:?}"
    );
    assert!(
        odd.iter().any(|d| d.message.contains("`p`") && matches!(d.level, Level::Error)),
        "the crash-tier proc check still fires under @allow: {diags:?}"
    );
}

#[test]
fn unquoted_allow_arg_is_loud() {
    // `@allow(layout.odd-item)` parses as arithmetic and can never match a
    // lint id — the natural typo must warn, not silently do nothing.
    let src = "\
module m
@allow(layout.odd-item)
data D: [u8; 1] = [1]
";
    let diags = full(src);
    assert!(
        diags.iter().any(|d| d.message.contains("[attr.allow-form]")),
        "expected [attr.allow-form]: {diags:?}"
    );
}

#[test]
fn as_compat_modules_are_exempt() {
    let src = "\
module m
@as_compat
data D: [u8; 1] = [1]
proc p () {
    rts
}
";
    let diags = full(src);
    assert!(odd_items(&diags).is_empty(), "@as_compat: the reference's placement is truth: {diags:?}");
}

// ---- 4. scripts count as procs (code by construction) -------------------------

#[test]
fn odd_script_is_error() {
    let src = "\
module m
newtype ScriptPc = u16
struct S (size: $24) {
    _pad0: [u8; $20],
    resume: ScriptPc @ $20,
    _pad1: [u8; 2] @ $22,
}
data Pad: [u8; 1] = [0]
script brain (a0: *S) (encoding: word_offsets) shows done {
    yield
}
proc done () { rts }
";
    let diags = full(src);
    assert!(
        odd_items(&diags)
            .iter()
            .any(|d| d.message.contains("`brain`") && matches!(d.level, Level::Error)),
        "an odd script is odd code (its table shifts everything odd): {diags:?}"
    );
}
