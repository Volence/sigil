//! Integration tests for `Value::Data` + the `Data` monoid + checked emission
//! (Spec 2, Plan 3 — T7): the `byte`/`bytes` builtins and `Data.empty`/`++`
//! monoid, the checked struct-literal (§4.5 / D-P3.12), and `lower_to_data` —
//! the CPU-neutral, structured, range-checked `DataBuf` that is the Plan 3/4
//! seam (D-P3.5). No endianness is committed and no pointer address is
//! resolved here; that is all Plan 4.
use sigil_frontend_emp::layout::eval_data;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{Cell, DataBuf};
use sigil_span::Diagnostic;

/// Parse `src` (asserting a clean parse) and lower the data item named `name`.
fn data(src: &str, name: &str) -> (Option<DataBuf>, Vec<Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_data(&file, name)
}

// ---- the Data monoid: Data.empty / byte / bytes / ++ --------------------

#[test]
fn data_monoid_concat_builds_cells() {
    let src = "module m\ndata D = Data.empty ++ byte(5) ++ bytes([1, 2, 3])\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 4);
    assert_eq!(
        buf.cells,
        vec![
            Cell::Scalar { value: 5, width: 1, signed: false },
            Cell::Bytes(vec![1, 2, 3]),
        ]
    );
}

#[test]
fn data_empty_alone_is_zero_bytes() {
    let (buf, diags) = data("module m\ndata D = Data.empty\n", "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 0);
    assert!(buf.cells.is_empty());
}

#[test]
fn byte_out_of_range_is_diagnosed() {
    let (buf, diags) = data("module m\ndata D = byte(300)\n", "D");
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("byte") && diags[0].message.contains("300"),
        "was {:?}",
        diags[0].message
    );
    // The failed byte poisons silently to an empty buffer (already reported).
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn bytes_element_out_of_range_is_diagnosed() {
    let (_buf, diags) = data("module m\ndata D = bytes([1, 2, 999])\n", "D");
    assert!(
        diags.iter().any(|d| d.message.contains("999")),
        "expected an out-of-range element diagnostic, got {diags:?}"
    );
}

// ---- array data items ---------------------------------------------------

#[test]
fn array_of_i8_lowers_to_signed_byte_scalars() {
    let src = "module m\ndata T: [i8; 4] = [1, -2, 3, -4]\n";
    let (buf, diags) = data(src, "T");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 4);
    assert_eq!(
        buf.cells,
        vec![
            Cell::Scalar { value: 1, width: 1, signed: true },
            Cell::Scalar { value: -2, width: 1, signed: true },
            Cell::Scalar { value: 3, width: 1, signed: true },
            Cell::Scalar { value: -4, width: 1, signed: true },
        ]
    );
}

#[test]
fn array_element_out_of_type_range_is_emit_error() {
    let src = "module m\ndata T: [i8; 4] = [1, 200, 3, 4]\n";
    let (_buf, diags) = data(src, "T");
    assert!(
        diags.iter().any(|d| d.message.contains("[emit.out-of-range]") && d.message.contains("200")),
        "expected an [emit.out-of-range] on 200, got {diags:?}"
    );
}

#[test]
fn array_wrong_length_is_diagnosed() {
    let src = "module m\ndata T: [i8; 4] = [1, 2, 3]\n";
    let (_buf, diags) = data(src, "T");
    assert!(
        diags.iter().any(|d| d.message.contains("length") || d.message.contains("expected 4")),
        "expected a length-mismatch diagnostic, got {diags:?}"
    );
}

// ---- checked struct literals + struct data items ------------------------

#[test]
fn struct_data_lowers_to_field_cells() {
    let src = "module m\nstruct S { a: u16, b: u8 }\ndata D: S = S{ a: 258, b: 7 }\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    // a: u16 @ 0 (2 bytes), b: u8 @ 2 (1 byte) → 3 bytes total.
    assert_eq!(buf.size, 3);
    assert_eq!(
        buf.cells,
        vec![
            Cell::Scalar { value: 258, width: 2, signed: false },
            Cell::Scalar { value: 7, width: 1, signed: false },
        ]
    );
}

#[test]
fn struct_missing_field_no_default_is_diagnosed() {
    let src = "module m\nstruct S { a: u8, b: u8 }\ndata D: S = S{ a: 1 }\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("[struct.missing-field]") && d.message.contains("b")),
        "expected a missing-field diagnostic for `b`, got {diags:?}"
    );
}

#[test]
fn struct_unknown_field_is_diagnosed() {
    let src = "module m\nstruct S { a: u8 }\ndata D: S = S{ a: 1, z: 2 }\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("field") && d.message.contains("z")),
        "expected an unknown-field diagnostic for `z`, got {diags:?}"
    );
}

#[test]
fn struct_default_fills_omitted_field() {
    let src = "module m\nstruct S { a: u8, b: u8 = 9 }\ndata D: S = S{ a: 1 }\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 2);
    assert_eq!(
        buf.cells,
        vec![
            Cell::Scalar { value: 1, width: 1, signed: false },
            Cell::Scalar { value: 9, width: 1, signed: false },
        ]
    );
}

#[test]
fn struct_size_mismatch_surfaces_through_data_item() {
    let src = "module m\nstruct S (size: 4) { a: u8, b: u8 }\ndata D: S = S{ a: 1, b: 2 }\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("declared size 4")),
        "expected a (size:) mismatch diagnostic, got {diags:?}"
    );
}

// ---- pointer fields: the Plan-4 SymRef seam -----------------------------

#[test]
fn pointer_field_lowers_to_symref() {
    let src = "module m\n\
               comptime fn init() -> u8 { 0 }\n\
               struct Obj { code: *u8, flags: u8 }\n\
               data D: Obj = Obj{ code: init, flags: 3 }\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 5);
    assert_eq!(
        buf.cells,
        vec![
            Cell::SymRef { name: "init".into(), width: 4, windowed: false },
            Cell::Scalar { value: 3, width: 1, signed: false },
        ]
    );
}

// ---- winptr: the §7.2 windowed bank pointer -----------------------------

#[test]
fn winptr_of_fn_ref_is_windowed_width2_symref() {
    // The happy path via the FnRef capture: `winptr(sfx)` → a one-cell
    // Value::Data holding a 2-byte windowed SymRef (D-P4.5 / §7.2).
    let src = "module m\n\
               comptime fn sfx() -> u8 { 0 }\n\
               data P = winptr(sfx)\n";
    let (buf, diags) = data(src, "P");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 2);
    assert_eq!(buf.cells, vec![Cell::SymRef { name: "sfx".into(), width: 2, windowed: true }]);
}

#[test]
fn winptr_of_string_uses_the_str_capture_path() {
    // `winptr("name")` captures the symbol name from a Value::Str (the second
    // capture path, mirroring `lower_ptr`), yielding the same width-2 windowed
    // SymRef.
    let src = "module m\ndata P = winptr(\"sfx_jump\")\n";
    let (buf, diags) = data(src, "P");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 2);
    assert_eq!(
        buf.cells,
        vec![Cell::SymRef { name: "sfx_jump".into(), width: 2, windowed: true }]
    );
}

#[test]
fn winptr_wrong_arity_is_diagnosed() {
    // Zero args and two args both trip the arity check.
    let (buf, diags) = data("module m\ndata P = winptr()\n", "P");
    assert!(
        diags.iter().any(|d| d.message.contains("winptr") && d.message.contains("1 argument")),
        "expected a winptr arity diagnostic, got: {diags:?}"
    );
    // A Poison result lowers to an empty buffer (no cells).
    assert!(buf.expect("data buf").cells.is_empty());

    let src = "module m\n\
               comptime fn a() -> u8 { 0 }\n\
               comptime fn b() -> u8 { 0 }\n\
               data P = winptr(a, b)\n";
    let (_buf, diags) = data(src, "P");
    assert!(
        diags.iter().any(|d| d.message.contains("winptr") && d.message.contains("1 argument")),
        "expected a winptr arity diagnostic for two args, got: {diags:?}"
    );
}

#[test]
fn winptr_non_symbol_arg_is_poison_and_diagnosed() {
    // A non-reference argument (an integer) cannot name a symbol: diagnostic +
    // Poison (→ empty buffer).
    let (buf, diags) = data("module m\ndata P = winptr(3)\n", "P");
    assert!(
        diags.iter().any(|d| d.message.contains("winptr") && d.message.contains("symbol reference")),
        "expected a winptr non-symbol diagnostic, got: {diags:?}"
    );
    assert!(buf.expect("data buf").cells.is_empty());
}

// ---- enum-typed fields --------------------------------------------------

#[test]
fn enum_field_lowers_to_discriminant_scalar() {
    let src = "module m\n\
               enum Dir: u8 { Up = 0, Down = 1, Left = 2 }\n\
               struct S { d: Dir, x: u8 }\n\
               data D: S = S{ d: Dir.Left, x: 7 }\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 2);
    assert_eq!(
        buf.cells,
        vec![
            Cell::Scalar { value: 2, width: 1, signed: false },
            Cell::Scalar { value: 7, width: 1, signed: false },
        ]
    );
}

// ---- type inference / errors --------------------------------------------

#[test]
fn data_missing_type_and_uninferable_is_diagnosed() {
    let (_buf, diags) = data("module m\ndata D = 5\n", "D");
    assert!(
        diags.iter().any(|d| d.message.contains("type")),
        "expected a cannot-infer-type diagnostic, got {diags:?}"
    );
}

#[test]
fn data_type_inferred_from_struct_literal() {
    // `T` omitted; the initializer names its type (§4.5).
    let src = "module m\nstruct S { a: u8, b: u8 }\ndata D = S{ a: 1, b: 2 }\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(buf.expect("data buf").size, 2);
}

#[test]
fn missing_data_item_is_diagnosed() {
    let (buf, diags) = data("module m\ndata D = byte(1)\n", "NOPE");
    assert!(buf.is_none());
    assert!(
        diags.iter().any(|d| d.message.contains("NOPE")),
        "expected a no-such-data diagnostic, got {diags:?}"
    );
}

// ---- T7 review: struct-default construction cycle guard -----------------

#[test]
fn self_referential_struct_default_does_not_crash() {
    // A field default that constructs its own struct would recurse forever
    // (pre-fix: stack overflow → SIGABRT). It must instead diagnose and stop.
    let src = "module m\nstruct A { x: A = A{} }\ndata D: A = A{}\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic struct construction")),
        "expected a cyclic-construction diagnostic, got {diags:?}"
    );
}

// ---- T7 review: annotation-size check on a Data initializer -------------

#[test]
fn data_annotation_size_mismatch_is_diagnosed() {
    let src = "module m\ndata D: [u8; 3] = bytes([1, 2])\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("[emit.size-mismatch]")),
        "expected a size-mismatch diagnostic, got {diags:?}"
    );
}

#[test]
fn data_annotation_size_match_is_clean() {
    let src = "module m\ndata D: [u8; 3] = bytes([1, 2, 3])\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(buf.expect("data buf").size, 3);
}

// ---- T7 review: fixed-point emission ------------------------------------

#[test]
fn fixed_value_emits_signed_scalar_at_byte_width() {
    // 65536 = 1.0 in fixed<16,16>; the store is a signed 4-byte scalar.
    let src = "module m\ndata D: fixed<16, 16> = 65536\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 4);
    assert_eq!(buf.cells, vec![Cell::Scalar { value: 65536, width: 4, signed: true }]);
}

#[test]
fn fixed_non_whole_byte_is_diagnosed() {
    let src = "module m\ndata D: fixed<4, 3> = 0\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("whole number of bytes")),
        "expected a whole-byte diagnostic, got {diags:?}"
    );
}

#[test]
fn fixed_too_wide_to_emit_is_diagnosed() {
    // fixed<32,32> = 8 bytes — no 68k data directive is 8 bytes wide.
    let src = "module m\ndata D: fixed<32, 32> = 0\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("too wide to store as a scalar")),
        "expected a too-wide diagnostic, got {diags:?}"
    );
}

// ---- T7 review: newtype / refined lowering ------------------------------

#[test]
fn newtype_lowers_at_underlying_width() {
    let src = "module m\nnewtype Word = u16\ndata D: Word = Word(258)\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.cells, vec![Cell::Scalar { value: 258, width: 2, signed: false }]);
}

#[test]
fn refined_lowers_at_underlying_width() {
    let src = "module m\ndata D: u8 where 0..200 = 50\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.cells, vec![Cell::Scalar { value: 50, width: 1, signed: false }]);
}

#[test]
fn refined_out_of_underlying_range_diagnoses_at_emission() {
    // 300 fits neither `u8` (the underlying store) — the emission range-check
    // fires even though the refinement bound was never construction-checked.
    let src = "module m\ndata D: u8 where 0..10 = 300\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("[emit.out-of-range]") && d.message.contains("300")),
        "expected an [emit.out-of-range] on 300, got {diags:?}"
    );
}

// ---- T7 review: tuple lowering ------------------------------------------

#[test]
fn tuple_lowers_each_element() {
    let src = "module m\ndata D: (u8, u16) = (1, 258)\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 3);
    assert_eq!(
        buf.cells,
        vec![
            Cell::Scalar { value: 1, width: 1, signed: false },
            Cell::Scalar { value: 258, width: 2, signed: false },
        ]
    );
}

#[test]
fn tuple_arity_mismatch_is_diagnosed() {
    let src = "module m\ndata D: (u8, u16) = (1, 2, 3)\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("tuple arity mismatch")),
        "expected a tuple arity diagnostic, got {diags:?}"
    );
}

// ---- T7 review: duplicate-field detection -------------------------------

#[test]
fn struct_duplicate_field_is_diagnosed() {
    let src = "module m\nstruct S { a: u8 }\ndata D: S = S{ a: 1, a: 2 }\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("more than once")),
        "expected a duplicate-field diagnostic, got {diags:?}"
    );
}
