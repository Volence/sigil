//! A typed comptime `const` binding's literal is arity-checked against its
//! declared type at ELABORATION, whether or not the value is ever emitted.
//!
//! The `data` path gets this from byte emission (`eval::emit`'s `lower_array`),
//! which a `const` never reaches — a `const` binds a comptime value and lowers to
//! zero bytes. The check therefore lives in its own once-per-compile validator
//! driven from the lowering funnel, so a wrong-arity array in a const refuses at
//! the declaration site with the SAME diagnostic wording the emitting path uses.
//!
//! The record-shape fixture below is the aeon `band_record` family: a struct
//! whose trailing capability tails are arrays sized by a comptime constant
//! (`BAND_EXT_N` / `BAND_CURVE_N`, both 0 in a no-new-capability game). A record
//! literal carrying a one-element `br_ext` against `BAND_EXT_N = 0` is the
//! one-unit shape poison for that family, and the array it gets wrong is NESTED
//! inside a struct field — so the check has to recurse into struct fields to see
//! it at all.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;

/// Lower `src` and return every diagnostic message.
fn diags(src: &str) -> Vec<String> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, ds) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    ds.into_iter().map(|d| d.message).collect()
}

/// The aeon `band_record` family, verbatim in shape: a legacy prefix plus two
/// capability-selected array tails whose lengths are comptime constants.
const BAND_DECLS: &str = "\
module m
pub const BAND_EXT_N = 0
pub const BAND_CURVE_N = 0
pub struct band_entry {
    band_top_plane:      u16,
    band_factor_a_s1:    u8,
    band_factor_a_s2:    u8,
    band_factor_b_s1:    u8,
    band_factor_b_s2:    u8,
    band_factor_ops:     u8,
    band_deform_shift_a: u8,
    band_deform_shift_b: u8,
    band_phase_offset:   u8,
}
pub struct band_ext (size: 10) {
    bx_deform_table_a: *u8,
    bx_deform_table_b: *u8,
    bx_deform_speed_a: u8,
    bx_deform_speed_b: u8,
}
pub struct band_curve (size: 10) {
    bc_to_s1:  u8,
    bc_to_s2:  u8,
    bc_flags:  u8,
    bc_pad:    u8,
    bc_step:   i16,
    bc_rem:    i16,
    bc_span:   u16,
}
pub struct band_record (size: sizeof(band_entry) + sizeof(band_ext) * BAND_EXT_N + sizeof(band_curve) * BAND_CURVE_N) {
    br_base:  band_entry,
    br_ext:   [band_ext; BAND_EXT_N],
    br_curve: [band_curve; BAND_CURVE_N],
}
";

/// A `band_entry` literal with every field supplied — the record's legacy prefix.
const BAND_BASE_LIT: &str = "band_entry{ band_top_plane: 0, band_factor_a_s1: 0, \
band_factor_a_s2: 0, band_factor_b_s1: 0, band_factor_b_s2: 0, band_factor_ops: 0, \
band_deform_shift_a: 0, band_deform_shift_b: 0, band_phase_offset: 0 }";

/// A `band_ext` literal with every field supplied — one extension element.
const BAND_EXT_LIT: &str = "band_ext{ bx_deform_table_a: 0, bx_deform_table_b: 0, \
bx_deform_speed_a: 0, bx_deform_speed_b: 0 }";

/// THE MEASURED GAP: a one-element `br_ext` against `BAND_EXT_N = 0`, bound to a
/// comptime `const`. Nothing emits it, so the byte path never sees it; the
/// declaration itself is wrong and must refuse.
#[test]
fn const_record_wrong_tail_arity_refuses() {
    let src = format!(
        "{BAND_DECLS}\nconst P: band_record = band_record{{ br_base: {BAND_BASE_LIT}, \
         br_ext: [ {BAND_EXT_LIT} ], br_curve: [] }}\n"
    );
    let msgs = diags(&src);
    assert!(
        msgs.iter().any(|m| m.contains("array length mismatch: expected 0 element(s), got 1")),
        "one-element br_ext against BAND_EXT_N = 0 must refuse: {msgs:?}"
    );
}

/// The defect-removed control: the same record with an EMPTY `br_ext` is the
/// shape a no-new-capability game lowers, and must stay clean.
#[test]
fn const_record_right_tail_arity_is_clean() {
    let src = format!(
        "{BAND_DECLS}\nconst P: band_record = band_record{{ br_base: {BAND_BASE_LIT}, \
         br_ext: [], br_curve: [] }}\n"
    );
    let msgs = diags(&src);
    assert!(msgs.is_empty(), "the capability-off record must lower clean: {msgs:?}");
}

/// The same defect one level simpler: a bare array-typed const, no struct in the
/// way. Both directions — too few and too many — name the declared length.
#[test]
fn const_bare_array_arity_both_directions() {
    let short = diags("module m\nconst A: [u8; 3] = [1, 2]\n");
    assert!(
        short.iter().any(|m| m.contains("array length mismatch: expected 3 element(s), got 2")),
        "too-few: {short:?}"
    );
    let long = diags("module m\nconst A: [u8; 3] = [1, 2, 3, 4]\n");
    assert!(
        long.iter().any(|m| m.contains("array length mismatch: expected 3 element(s), got 4")),
        "too-many: {long:?}"
    );
    let exact = diags("module m\nconst A: [u8; 3] = [1, 2, 3]\n");
    assert!(exact.is_empty(), "exact length must be clean: {exact:?}");
}

/// An array of arrays: the inner rows are checked too, not just the outer count.
#[test]
fn const_nested_array_rows_are_checked() {
    let msgs = diags("module m\nconst A: [[u8; 2]; 2] = [[1, 2], [3, 4, 5]]\n");
    assert!(
        msgs.iter().any(|m| m.contains("array length mismatch: expected 2 element(s), got 3")),
        "inner row arity must be checked: {msgs:?}"
    );
}

/// An array reached through a tuple element.
#[test]
fn const_array_inside_tuple_is_checked() {
    let msgs = diags("module m\nconst A: (u8, [u8; 2]) = (1, [2, 3, 4])\n");
    assert!(
        msgs.iter().any(|m| m.contains("array length mismatch: expected 2 element(s), got 3")),
        "tuple-nested array arity must be checked: {msgs:?}"
    );
}

/// A `const` inside a `section {}` block lives in the same flat namespace as a
/// top-level one (§7.1) and is checked identically.
#[test]
fn const_inside_section_is_checked() {
    let msgs = diags(
        "module m\nsection s (cpu: m68000) {\n    const A: [u8; 2] = [1, 2, 3]\n}\n",
    );
    assert!(
        msgs.iter().any(|m| m.contains("array length mismatch: expected 2 element(s), got 3")),
        "section-nested const must be checked: {msgs:?}"
    );
}

/// A string literal against a `[u8; n]` const carries the same arity contract the
/// emitting path gives it: the author sizes `n`, there is no implicit terminator.
#[test]
fn const_string_byte_array_length_is_checked() {
    let msgs = diags("module m\nconst S: [u8; 4] = \"HELLO\"\n");
    assert!(
        msgs.iter().any(|m| m.contains("array length mismatch: expected 4 element(s), got 5")),
        "string-as-byte-array length must be checked: {msgs:?}"
    );
    let exact = diags("module m\nconst S: [u8; 5] = \"HELLO\"\n");
    assert!(exact.is_empty(), "exact-length string must be clean: {exact:?}");
}

/// An UNTYPED const is unconstrained — no annotation, nothing to check against.
#[test]
fn untyped_const_is_unconstrained() {
    let msgs = diags("module m\nconst A = [1, 2, 3]\n");
    assert!(msgs.is_empty(), "an untyped const has no arity contract: {msgs:?}");
}

/// A const whose value cannot be evaluated in this module reports through its
/// own use site, not as a spurious arity complaint from the validator.
#[test]
fn unresolvable_const_value_yields_no_arity_diagnostic() {
    let msgs = diags("module m\nconst A: [u8; 2] = NOT_A_NAME\n");
    assert!(
        !msgs.iter().any(|m| m.contains("array length mismatch")),
        "a poisoned value must not produce an arity diagnostic: {msgs:?}"
    );
}
