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
            Cell::Scalar { value: 5, width: 1, signed: false, le: false },
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

#[test]
fn binary_percent_literal_emits_the_right_byte() {
    // `%10100101` (lexical gaps, Task 1) end-to-end through parse+eval+lower.
    let src = "module m\ndata D = byte(%10100101)\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 1);
    assert_eq!(buf.cells, vec![Cell::Scalar { value: 0b10100101, width: 1, signed: false, le: false }]);
}

#[test]
fn char_literal_emits_the_right_byte() {
    // `'A'` (lexical gaps, Task 3) is raw ASCII 65 end-to-end through
    // parse+eval+lower — a plain integer, no charmap involved.
    let src = "module m\ndata D = byte('A')\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 1);
    assert_eq!(buf.cells, vec![Cell::Scalar { value: 0x41, width: 1, signed: false, le: false }]);
}

// ---- string literals in data position (lexical gaps, Task 4) ------------
//
// Ratified decision: string/char literals default to RAW ASCII bytes, and
// termination is AUTHOR-CONTROLLED — `bytes(...)` NEVER emits an implicit
// trailing 0. `bytes("HELLO")` is exactly the 5 ASCII bytes; a terminator is
// only present if the author writes one (`++ byte(0)`, or the `\0` escape).

#[test]
fn bytes_of_string_emits_raw_ascii_no_terminator() {
    let src = r#"module m
data D = bytes("HELLO")
"#;
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 5, "must NOT include an implicit trailing 0");
    assert_eq!(buf.cells, vec![Cell::Bytes(vec![0x48, 0x45, 0x4C, 0x4C, 0x4F])]);
}

#[test]
fn bytes_of_string_composes_with_an_explicit_terminator() {
    // Author-controlled termination: `++ byte(0)` is how you ask for one.
    let src = r#"module m
data D = bytes("HI") ++ byte(0)
"#;
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 3);
    assert_eq!(
        buf.cells,
        vec![
            Cell::Bytes(vec![0x48, 0x49]),
            Cell::Scalar { value: 0, width: 1, signed: false, le: false },
        ]
    );
}

#[test]
fn bytes_of_string_with_null_escape_emits_explicit_terminator() {
    // The `\0` escape (Task 4 part 3) is the other author-controlled route to
    // the same terminator byte, folded straight into the string content.
    let src = r#"module m
data D = bytes("HELLO\0")
"#;
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 6);
    assert_eq!(buf.cells, vec![Cell::Bytes(vec![0x48, 0x45, 0x4C, 0x4C, 0x4F, 0x00])]);
}

#[test]
fn bytes_of_non_ascii_string_is_diagnosed() {
    // `bytes("é")` must NOT silently emit UTF-8 bytes — ASCII-only, same rule
    // as the Task 3 char literal.
    let src = "module m\ndata D = bytes(\"\u{e9}\")\n"; // "é"
    let (buf, diags) = data(src, "D");
    assert!(!diags.is_empty(), "expected an ASCII-only diagnostic, got none");
    assert!(
        diags.iter().any(|d| d.message.to_lowercase().contains("ascii")),
        "expected an ASCII-only diagnostic, got {diags:?}"
    );
    // The failed conversion poisons silently to an empty buffer.
    assert_eq!(buf.expect("data buf").size, 0);
}

#[test]
fn bytes_of_array_is_unchanged_by_the_string_addition() {
    // Existing array behavior must be untouched.
    let src = "module m\ndata D = bytes([1, 2, 3])\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 3);
    assert_eq!(buf.cells, vec![Cell::Bytes(vec![1, 2, 3])]);
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
            Cell::Scalar { value: 1, width: 1, signed: true, le: false },
            Cell::Scalar { value: -2, width: 1, signed: true, le: false },
            Cell::Scalar { value: 3, width: 1, signed: true, le: false },
            Cell::Scalar { value: -4, width: 1, signed: true, le: false },
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
            Cell::Scalar { value: 258, width: 2, signed: false, le: false },
            Cell::Scalar { value: 7, width: 1, signed: false, le: false },
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
    // S2-D13(h): elision is an EXPLICIT act — `..` opts into default fill.
    let src = "module m\nstruct S { a: u8, b: u8 = 9 }\ndata D: S = S{ a: 1, .. }\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 2);
    assert_eq!(
        buf.cells,
        vec![
            Cell::Scalar { value: 1, width: 1, signed: false, le: false },
            Cell::Scalar { value: 9, width: 1, signed: false, le: false },
        ]
    );
}


#[test]
fn omitted_defaulted_field_without_rest_is_error() {
    // Without `..`, a defaulted field may not be silently elided — the page
    // must show the elision (S2-D13(h) tightening, recorded in the tranche-0
    // notes for the checkpoint).
    let src = "module m\nstruct S { a: u8, b: u8 = 9 }\ndata D: S = S{ a: 1 }\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("[struct.missing-field]")
            && d.message.contains("b")
            && d.message.contains("..")),
        "the error names the field AND offers the `..` spelling: {diags:?}"
    );
}

#[test]
fn rest_does_not_cover_defaultless_fields() {
    let src = "module m\nstruct S { a: u8, b: u8 = 9 }\ndata D: S = S{ .. }\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("[struct.missing-field]") && d.message.contains("a")),
        "`..` fills defaults only — `a` has none and stays required: {diags:?}"
    );
}

#[test]
fn rest_with_nothing_to_fill_is_harmless() {
    // All fields given + `..` — legal (refactor-friendly: deleting the last
    // defaulted field from a literal shouldn't force deleting the marker).
    let src = "module m\nstruct S { a: u8, b: u8 = 9 }\ndata D: S = S{ a: 1, b: 2, .. }\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(buf.expect("buf").size, 2);
}


#[test]
fn rest_on_a_bitfield_literal_warns() {
    // Bitfields keep their long-standing omitted-fields-are-0 semantics
    // (checkpoint question recorded); a `..` there is a no-op and says so.
    let src = "module m\nbitfield B: u8 { x: 4, y: 4 }\ndata D: u8 = B{ x: 1, .. }\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("no effect") && d.message.contains("bitfield")),
        "expected the bitfield no-effect warning: {diags:?}"
    );
}

#[test]
fn rest_on_an_undeclared_type_warns() {
    let src = "module m\ndata D: u8 = Foo{ a: 1, .. }\n";
    let (_buf, diags) = data(src, "D");
    assert!(
        diags.iter().any(|d| d.message.contains("no effect") && d.message.contains("Foo")),
        "expected the undeclared-type no-effect warning: {diags:?}"
    );
}

#[test]
fn rest_not_last_is_a_parse_error() {
    let (_, perrs) = sigil_frontend_emp::parse_str(
        "module m\nstruct S { a: u8, b: u8 = 9 }\ndata D: S = S{ .., a: 1 }\n",
    );
    assert!(
        perrs.iter().any(|d| d.message.contains("`..`")),
        "`..` must be the literal's last member: {perrs:?}"
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
            Cell::Scalar { value: 3, width: 1, signed: false, le: false },
        ]
    );
}

// ---- winptr: the §7.2 windowed bank pointer -----------------------------

/// The residual tree `winptr(sym)` now yields (R-T0.5): `(sym & $7FFF) | $8000`.
fn winptr_tree(name: &str) -> sigil_ir::expr::Expr {
    use sigil_ir::expr::{BinOp, Expr};
    Expr::Binary {
        op: BinOp::Or,
        lhs: Box::new(Expr::Binary {
            op: BinOp::And,
            lhs: Box::new(Expr::Sym(name.into())),
            rhs: Box::new(Expr::Int(0x7FFF)),
        }),
        rhs: Box::new(Expr::Int(0x8000)),
    }
}

#[test]
fn winptr_of_fn_ref_is_windowed_link_expr_cell() {
    // The happy path via the FnRef capture (R-T0.5): `winptr(sfx)` is now a
    // Value::LinkExpr `(sfx & $7FFF) | $8000`. Emitted into a `u16` field it
    // lowers to a general link-expr VALUE cell (`Cell::Expr`, width 2), whose
    // Value16Be/Value16Le fixup writes IDENTICAL bytes to the old windowed
    // SymRef (proven end-to-end in lower_sections.rs).
    let src = "module m\n\
               comptime fn sfx() -> u8 { 0 }\n\
               data P: u16 = winptr(sfx)\n";
    let (buf, diags) = data(src, "P");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 2);
    assert_eq!(buf.cells, vec![Cell::Expr { expr: winptr_tree("sfx"), width: 2, le: false }]);
}

#[test]
fn winptr_of_string_uses_the_str_capture_path() {
    // `winptr("name")` captures the symbol name from a Value::Str (the second
    // capture path), yielding the same windowed link-expr VALUE cell.
    let src = "module m\ndata P: u16 = winptr(\"sfx_jump\")\n";
    let (buf, diags) = data(src, "P");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 2);
    assert_eq!(buf.cells, vec![Cell::Expr { expr: winptr_tree("sfx_jump"), width: 2, le: false }]);
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
            Cell::Scalar { value: 2, width: 1, signed: false, le: false },
            Cell::Scalar { value: 7, width: 1, signed: false, le: false },
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
    // (`..` on both literals: under S2-D13(h) a default only evaluates via
    // the explicit rest marker, and the recursion lives in the default.)
    let src = "module m\nstruct A { x: A = A{ .. } }\ndata D: A = A{ .. }\n";
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
    assert_eq!(buf.cells, vec![Cell::Scalar { value: 65536, width: 4, signed: true, le: false }]);
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
    assert_eq!(buf.cells, vec![Cell::Scalar { value: 258, width: 2, signed: false, le: false }]);
}

#[test]
fn refined_lowers_at_underlying_width() {
    let src = "module m\ndata D: u8 where 0..200 = 50\n";
    let (buf, diags) = data(src, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.cells, vec![Cell::Scalar { value: 50, width: 1, signed: false, le: false }]);
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
            Cell::Scalar { value: 1, width: 1, signed: false, le: false },
            Cell::Scalar { value: 258, width: 2, signed: false, le: false },
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
