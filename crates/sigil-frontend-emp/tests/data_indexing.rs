//! Integration tests for comptime indexing + element-typed Data views
//! (Spec 2, D2.33 — the tranche-4 opening build).
//!
//! Two features, one decision record:
//!
//! 1. **Comptime indexing** — postfix `base[i]` in comptime expressions:
//!    `Value::Array` yields the element; `Value::Data` yields the raw BYTE at
//!    that offset as an int (only where the byte is comptime-known: `Bytes`
//!    runs and width-1 scalars — a multi-byte scalar's byte order and a
//!    symbol reference's value are committed at link, so indexing them is the
//!    loud `[index.uncommitted-byte]`). Postfix `.field` also generalizes off
//!    bare paths, so `embed(...).len` reads directly. Out-of-bounds /
//!    negative / non-int indexes are `[index.out-of-bounds]` / type errors —
//!    never a wrap, never a silent zero.
//!
//! 2. **Element-typed Data views** — `data X: [i16; N] = embed(...)`: the
//!    declared element type is a CHECK over the raw bytes (byte length must
//!    equal `N × sizeof(elem)`), and emission is BYTE-IDENTICAL to the
//!    untyped `data X = embed(...)` (the buffer passes through verbatim — no
//!    decode/re-encode, so the file's big-endian words survive any section).
//!    `le`-typed elements are rejected (the view documents BIG-endian file
//!    bytes); non-scalar elements are rejected with steering.
use sigil_frontend_emp::eval::{Env, Evaluator};
use sigil_frontend_emp::layout::eval_data_with_root;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::parser::parse_expr_for_tests;
use sigil_frontend_emp::value::{Cell, DataBuf, Value};
use sigil_ir::backend::Cpu;
use sigil_span::Diagnostic;
use std::path::{Path, PathBuf};

/// `tests/vectors/`, containing `embed_fixture.bin` (bytes `0x00..=0x0B`).
fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("vectors")
}

/// Parse `src` (asserting a clean parse) and evaluate the data item `name`
/// with the sandbox rooted at [`vectors_dir`].
fn data(src: &str, name: &str) -> (Option<DataBuf>, Vec<Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (buf, _asserts, ds) = eval_data_with_root(&file, name, None, Some(&vectors_dir()), &[]);
    (buf, ds)
}

/// Parse + FULL-lower `src` (all items, incl. module-level `ensure`s), with
/// the sandbox rooted at [`vectors_dir`] — the `lower_data.rs` house helper.
fn lower_all(src: &str) -> Vec<Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(vectors_dir()),
            embed_base: None,
            defines: vec![],
        },
    );
    diags
}

/// Parse+evaluate a bare expression (no sandbox — array/scalar cases).
fn eval(src: &str) -> (Value, Vec<Diagnostic>) {
    let e = parse_expr_for_tests(src);
    let mut ev = Evaluator::new();
    let mut env = Env::new();
    let v = ev.eval_expr(&e, &mut env);
    (v, ev.diags)
}

fn ok(src: &str, want: Value) {
    let (v, diags) = eval(src);
    assert_eq!(v, want, "value for `{src}`");
    assert!(diags.is_empty(), "expected no diagnostics for `{src}`, got {diags:?}");
}

fn poison_with(src: &str, needle: &str) {
    let (v, diags) = eval(src);
    assert_eq!(v, Value::Poison, "expected Poison for `{src}`");
    assert!(
        diags.iter().any(|d| d.message.contains(needle)),
        "diagnostic for `{src}` was {diags:?}, expected to contain {needle:?}"
    );
}

// ---- array indexing (pure exprs) ---------------------------------------

#[test]
fn array_index_reads_the_element() {
    ok("[10, 20, 30][1]", Value::Int(20));
}

#[test]
fn array_index_chains() {
    ok("[[1, 2], [3, 4]][1][0]", Value::Int(3));
}

#[test]
fn array_index_binds_tighter_than_arithmetic() {
    ok("[10, 20][0] + [1, 2][1]", Value::Int(12));
}

#[test]
fn parenthesized_base_indexes() {
    ok("([7, 8])[1]", Value::Int(8));
}

#[test]
fn array_index_out_of_bounds_is_loud() {
    poison_with("[1, 2][2]", "[index.out-of-bounds]");
}

#[test]
fn huge_index_is_out_of_bounds_not_a_wrap() {
    // Review C1: an index ≥ 2^64 must bounds-fail in the i128 domain — a
    // usize truncation would silently wrap `1 << 64` to element 0.
    poison_with("[10, 20][1 << 64]", "[index.out-of-bounds]");
    poison_with("[10, 20][(1 << 64) + 1]", "[index.out-of-bounds]");
    poison_with("bytes([9, 8, 7])[1 << 64]", "[index.out-of-bounds]");
}

#[test]
fn poison_base_propagates_silently() {
    // An unknown base name is already-reported — the index adds nothing.
    let (v, diags) = eval("nosuch[0]");
    assert_eq!(v, Value::Poison);
    assert_eq!(diags.len(), 1, "only the unknown-name diagnostic: {diags:?}");
    assert!(diags[0].message.contains("unknown name"));
}

#[test]
fn postfix_field_reads_struct_fields_off_index_results() {
    // `field_or_len`'s struct arm through the postfix Field node (not just
    // `.len`): index into an array of structs, then read a field.
    let src = "module m\n\
               struct P { x: u8, y: u8 }\n\
               const PTS = [P{ x: 1, y: 2 }, P{ x: 3, y: 4 }]\n\
               data X: [u8; 1] = [PTS[1].y]\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        buf.expect("data buf").cells,
        vec![Cell::Scalar { value: 4, width: 1, signed: false, le: false }]
    );
}

#[test]
fn poisoned_view_element_reports_once() {
    // Review I2: an unknown element type is already-reported by the resolve;
    // the view policing and the (bogus 0-byte) size check stay silent.
    let src = "module m\ndata X: [Bad; 4] = embed(\"embed_fixture.bin\")\n";
    let (_buf, diags) = data(src, "X");
    assert_eq!(diags.len(), 1, "exactly the unknown-type diagnostic: {diags:?}");
    assert!(diags[0].message.contains("Bad"));
}

#[test]
fn array_index_negative_is_loud() {
    poison_with("[1, 2][0 - 1]", "[index.out-of-bounds]");
}

#[test]
fn non_int_index_is_loud() {
    poison_with("[1, 2][\"a\"]", "index must be a comptime integer");
}

#[test]
fn non_indexable_base_is_loud() {
    poison_with("5[0]", "not indexable");
}

// ---- postfix `.len` off non-path bases ----------------------------------

#[test]
fn array_literal_len_reads_postfix() {
    ok("[1, 2, 3].len", Value::Int(3));
}

#[test]
fn parenthesized_range_len_reads_postfix() {
    ok("(0..4).len", Value::Int(4));
}

// ---- Data indexing (embed, through the data-item route) -----------------

#[test]
fn embed_byte_indexes_through_a_const_blob() {
    // The R7m.7 house pattern for a value-readable blob is a CONST (data
    // items stay link-position-only); fixture byte 3 is 0x03 — landed in a
    // [u8;1] so the value is visible in the emitted cell.
    let src = "module m\n\
               const Src = embed(\"embed_fixture.bin\")\n\
               data X: [u8; 1] = [Src[3]]\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(
        buf.cells,
        vec![Cell::Scalar { value: 3, width: 1, signed: false, le: false }]
    );
}

#[test]
fn embed_call_indexes_directly() {
    let src = "module m\n\
               data X: [u8; 2] = [embed(\"embed_fixture.bin\")[0], embed(\"embed_fixture.bin\")[11]]\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(
        buf.cells,
        vec![
            Cell::Scalar { value: 0, width: 1, signed: false, le: false },
            Cell::Scalar { value: 11, width: 1, signed: false, le: false },
        ]
    );
}

#[test]
fn embed_call_len_reads_postfix() {
    let src = "module m\n\
               data X: [u8; 1] = [embed(\"embed_fixture.bin\").len]\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(
        buf.cells,
        vec![Cell::Scalar { value: 12, width: 1, signed: false, le: false }]
    );
}

#[test]
fn data_index_out_of_bounds_is_loud() {
    let src = "module m\n\
               data X: [u8; 1] = [embed(\"embed_fixture.bin\")[12]]\n";
    let (_buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[index.out-of-bounds]")),
        "expected [index.out-of-bounds], got {diags:?}"
    );
}

#[test]
fn data_bytes_cells_index_as_pure_exprs() {
    // `bytes([...])` builds the same raw Bytes cells `embed` does.
    ok("bytes([9, 8, 7])[1]", Value::Int(8));
}

#[test]
fn byte_at_refuses_symbolic_cells() {
    // A symbol-reference cell's bytes are folded at link — `byte_at` (the
    // eval-path `[index.uncommitted-byte]` gate) refuses them. Every
    // expr-position Data source today (embed/bytes/byte/++) builds raw
    // Bytes cells, so this arm is defensive depth exercised directly.
    let mut buf = DataBuf::empty();
    buf.push(Cell::Bytes(vec![1]));
    buf.push(Cell::SymRef { name: "S".into(), width: 4, windowed: false });
    assert_eq!(buf.byte_at(0), Some(1), "the raw byte before the SymRef reads");
    for off in 1..5 {
        assert_eq!(buf.byte_at(off), None, "SymRef byte {off} is link-folded");
    }
}

#[test]
fn byte_at_reads_width1_scalars_and_refuses_wider() {
    // `DataBuf::byte_at` directly: a width-1 scalar is order-free (its
    // two's-complement low byte IS the emitted byte — i8 -1 reads 0xFF); a
    // multi-byte scalar's byte order is committed at stream time, so its
    // bytes are not comptime-known.
    let mut buf = DataBuf::empty();
    buf.push(Cell::Scalar { value: -1, width: 1, signed: true, le: false });
    buf.push(Cell::Scalar { value: 258, width: 2, signed: false, le: false });
    assert_eq!(buf.byte_at(0), Some(0xFF), "i8 -1 is the byte 0xFF");
    assert_eq!(buf.byte_at(1), None, "first byte of a u16 scalar is order-committed");
    assert_eq!(buf.byte_at(2), None, "second byte of a u16 scalar is order-committed");
    assert_eq!(buf.byte_at(3), None, "past the end");
}

// ---- element-typed Data views -------------------------------------------

#[test]
fn typed_word_view_is_byte_identical_to_the_raw_embed() {
    // 12 fixture bytes = 6 big-endian words: the check passes and the
    // buffer is the raw Bytes cell, verbatim — byte-identity.
    let src = "module m\ndata X: [i16; 6] = embed(\"embed_fixture.bin\")\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 12);
    assert_eq!(
        buf.cells,
        vec![Cell::Bytes((0u8..=11).collect())],
        "the view must pass the raw bytes through verbatim"
    );
}

#[test]
fn typed_long_view_checks_and_passes_through() {
    let src = "module m\ndata X: [u32; 3] = embed(\"embed_fixture.bin\")\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(buf.expect("data buf").size, 12);
}

#[test]
fn typed_byte_view_checks_and_passes_through() {
    let src = "module m\ndata X: [u8; 12] = embed(\"embed_fixture.bin\")\n";
    let (buf, diags) = data(src, "X");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(buf.expect("data buf").size, 12);
}

#[test]
fn typed_view_length_mismatch_is_loud() {
    // 12 bytes is not 5 words: the existing whole-item size check fires,
    // naming both byte counts (declared 10, produced 12).
    let src = "module m\ndata X: [u16; 5] = embed(\"embed_fixture.bin\")\n";
    let (_buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[emit.size-mismatch]")
            && d.message.contains("12")
            && d.message.contains("10")),
        "expected an [emit.size-mismatch] naming 12 vs 10 bytes, got {diags:?}"
    );
}

#[test]
fn typed_view_indivisible_length_is_loud() {
    // 12 bytes over [u32; 4] wants 16 — same size check, long elements.
    let src = "module m\ndata X: [u32; 4] = embed(\"embed_fixture.bin\")\n";
    let (_buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[emit.size-mismatch]")),
        "expected [emit.size-mismatch], got {diags:?}"
    );
}

#[test]
fn typed_view_le_element_is_rejected() {
    // The view documents BIG-endian file bytes (D2.33); a u16le view would
    // lie about them. Loud, with steering.
    let src = "module m\ndata X: [u16le; 6] = embed(\"embed_fixture.bin\")\n";
    let (_buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[data.view-le]")),
        "expected [data.view-le], got {diags:?}"
    );
}

#[test]
fn typed_view_non_scalar_element_is_rejected() {
    let src = "module m\n\
               struct P { x: u8, y: u8 }\n\
               data X: [P; 6] = embed(\"embed_fixture.bin\")\n";
    let (_buf, diags) = data(src, "X");
    assert!(
        diags.iter().any(|d| d.message.contains("[data.view-elem]")),
        "expected [data.view-elem], got {diags:?}"
    );
}

// ---- guards: the first consumer shape ------------------------------------

#[test]
fn sine_style_content_assert_passes_and_fails_correctly() {
    // The tranche-4 consumer shape: an `ensure` over embedded bytes (the
    // sine-table content asserts). Fixture byte 0 is 0x00 — the passing
    // guard is silent, the failing one is loud and interpolates the byte.
    let src = "module m\n\
               const Src = embed(\"embed_fixture.bin\")\n\
               ensure(Src[0] == 0, \"sine[0] must be 0, got {Src[0]}\")\n\
               data X: [u8; 1] = [0]\n";
    let diags = lower_all(src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "a true content assert must be silent: {diags:?}"
    );

    let src_bad = "module m\n\
                   const Src = embed(\"embed_fixture.bin\")\n\
                   ensure(Src[3] == 1, \"sine[3] must be 1, got {Src[3]}\")\n\
                   data X: [u8; 1] = [0]\n";
    let diags = lower_all(src_bad);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("sine[3] must be 1, got 3")),
        "a false content assert must fail loudly with the interpolated byte, got {diags:?}"
    );
}
