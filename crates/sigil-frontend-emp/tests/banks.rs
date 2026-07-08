//! Plan 7 #7-main Task 1 — the `bank:` section attribute (R7m.1): parses
//! beside `cpu:`/`vma:`, evaluates as a comptime positive power-of-two
//! integer, and threads to `ir::Section.bank`. The field is INERT here —
//! nothing reads it until Task 2's placement seam.

use sigil_frontend_emp::layout::eval_data_with_root;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, Section, SymbolTable};
use std::path::{Path, PathBuf};

/// Find a section by name in a lowered module (mirrors `lower_sections.rs`'s
/// helper — the default `text` sections are interleaved between placed ones).
fn section<'a>(module: &'a Module, name: &str) -> &'a Section {
    module
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section `{name}` in {:?}", module.sections.iter().map(|s| &s.name).collect::<Vec<_>>()))
}

#[test]
fn bank_attr_threads_to_section_bank() {
    let src = "module m\n\
               section s (bank: $8000) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(diags.is_empty(), "lower: {diags:?}");
    let s = section(&module, "s");
    assert_eq!(s.bank, Some(0x8000));
}

#[test]
fn bank_attr_composes_with_cpu_and_vma_in_any_order() {
    let src = "module m\n\
               section s (vma: $8000, bank: $4000, cpu: z80) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(diags.is_empty(), "lower: {diags:?}");
    let s = section(&module, "s");
    assert_eq!(s.bank, Some(0x4000));
    assert_eq!(s.vma_base, Some(0x8000));
    assert_eq!(s.cpu, Cpu::Z80);
}

#[test]
fn section_without_bank_attr_has_none() {
    let src = "module m\n\
               section s (vma: $8000) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(diags.is_empty(), "lower: {diags:?}");
    let s = section(&module, "s");
    assert_eq!(s.bank, None);
}

#[test]
fn bank_attr_non_power_of_two_is_diagnosed() {
    let src = "module m\n\
               section s (bank: 3) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        diags.iter().any(|d| d.message
            == "section `s` `bank:` must be a positive power-of-two comptime integer"),
        "expected the R7m.1 bank: diagnostic, got: {diags:?}"
    );
}

#[test]
fn bank_attr_zero_is_diagnosed() {
    let src = "module m\n\
               section s (bank: 0) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        diags.iter().any(|d| d.message
            == "section `s` `bank:` must be a positive power-of-two comptime integer"),
        "expected the R7m.1 bank: diagnostic, got: {diags:?}"
    );
}

// ---- Task 3: general link-expr data cells (Cell::Expr + ValueN kinds, S2-D13f)
//
// A `Value::LinkExpr` landing in a data cell of declared width w ∈ {1,2,4} now
// lowers to `Cell::Expr` → a width/CPU-selected VALUE fixup folded at link and
// unsigned-window range-checked on write (R7m.4). A provisional here() is minted
// by a `jbra` to a far label; arithmetic on it (`here() + N`, `here() >> N`)
// produces the residual LinkExpr these tests emit. ------------------------------

/// Lower `src` requiring no lower-time errors (mirrors here_provisional.rs).
fn lower_ok(src: &str) -> Module {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    let errs: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errs.is_empty(), "lower errors: {errs:?}");
    m
}

/// Full compile+link path: resolve_layout then link, return one section's bytes.
fn linked_bytes(m: &Module, section: &str) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(section).map(|s| s.bytes.clone()).unwrap_or_default()
}

/// (a) An arithmetic LinkExpr (`here() + 2`) emitted at width 2 in a 68k section
/// produces `Value16Be` bytes of the FINAL folded value — byte-asserted through
/// the full resolve_layout+link path. The jbra grows to bra.w (4 bytes), so the
/// data item H sits at $8000 + 4 = $8004; here()+2 folds to $8006, big-endian.
#[test]
fn link_expr_width2_68k_folds_big_endian() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 proc p () {\n\
                   jbra Far\n\
                 }\n\
                 data H: u16 = here() + 2\n\
                 data Pad = bytes(for i in 0..200 { 0 })\n\
                 proc Far () {\n\
                   rts\n\
                 }\n\
               }\n";
    let m = lower_ok(src);
    let bytes = linked_bytes(&m, "s");
    // bytes[0..4] = jbra bra.w; bytes[4..6] = H = ($8004 + 2) big-endian.
    assert_eq!(&bytes[4..6], &[0x80, 0x06], "here()+2 must fold to $8006 BE; got {:02X?}", &bytes[4..6]);
}

/// (b) Width 1 works (Value8): `here() >> 15` — the bank-id shift — into a u8
/// cell. H at $8004; $8004 >> 15 == 1.
#[test]
fn link_expr_width1_emits_value8() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 proc p () {\n\
                   jbra Far\n\
                 }\n\
                 data H: u8 = here() >> 15\n\
                 data Pad = bytes(for i in 0..200 { 0 })\n\
                 proc Far () {\n\
                   rts\n\
                 }\n\
               }\n";
    let m = lower_ok(src);
    let bytes = linked_bytes(&m, "s");
    assert_eq!(bytes[4], 0x01, "here()>>15 at $8004 must fold to 1; got {:#X}", bytes[4]);
}

/// (c) A fold overflowing the width window is an Error naming the cell and the
/// value. A u16 cell holding `here() + $8000` folds to $8004 + $8000 = $10004,
/// which is ≥ $10000 (does not fit an unsigned 16-bit cell).
#[test]
fn link_expr_overflow_is_range_error() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 proc p () {\n\
                   jbra Far\n\
                 }\n\
                 data H: u16 = here() + $8000\n\
                 data Pad = bytes(for i in 0..200 { 0 })\n\
                 proc Far () {\n\
                   rts\n\
                 }\n\
               }\n";
    let m = lower_ok(src);
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let err = sigil_link::link(&resolved, &SymbolTable::new()).expect_err("link must fail");
    assert!(
        err.iter().any(|d| d.message.contains("[value.out-of-range]")
            && d.message.contains("65540") // $10004
            && d.message.contains("16-bit")),
        "expected an unsigned-window range error naming the value, got: {err:?}"
    );
}

// (d) A Z80 section width-2 LinkExpr cell writes LITTLE-endian (Value16Le) — the
// R7m.5 Z80 probe. Z80 has no `jbra` (no relaxable → no provisional here() in a
// Z80 section until Task 4's `bankid`), so the CPU→endianness selection is
// proven at the two seams it actually lives in:
//   - the FRONTEND `stream_data` selection (a Z80 `Cell::Expr` → Value16Le), a
//     unit test in `lower/data.rs`;
//   - the LINKER `apply_fixup` write (Value16Le folds LE), a unit test in
//     `sigil-link/src/lib.rs`.
// Both are in-crate `#[cfg(test)]` modules (they touch pub(super)/private items);
// this comment records where the (d) evidence lives per the Task-3 plan.

// ---- Task 4: bankid() builtin + embed .len (R7m.3 / R7m.7) ------------------
//
// `bankid(L)` yields `Value::LinkExpr(((Sym(L) & $7F8000) >> 15))` — the Genesis
// cartridge bank id, a link-time value on the D2.23 machinery (D7.3). Emission
// rides Task 3's Cell::Expr; `ensure` over it defers to a LinkAssert with ZERO
// new code (D-H.4); a comptime-required position refuses via the existing
// reject_if_provisional choke point, steered by `[bank.provisional]` (R7m.3).
// `embed(...).len` is the comptime byte length of an embedded blob (R7m.7).

/// The fixture directory `embed` resolves paths against: `tests/vectors/`,
/// holding the deterministic `embed_fixture.bin` (the bytes 0x00..=0x0B, 12 B —
/// shared with `sandbox_embed.rs`).
fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("vectors")
}

/// Resolve layout, then evaluate the module's deferred `LinkAssert`s against the
/// final symbol table (the CLI compile path — `linked_bytes` does not run the
/// asserts). Returns the assertion diagnostics.
fn link_assert_diags(m: &Module) -> Vec<sigil_span::Diagnostic> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &m.link_asserts)
}

// ---- R7m.7: embed(...).len ---------------------------------------------------

/// RECON VERDICT (R7m.7): `.len` on a `Value::Data` receiver did NOT work before
/// this task — neither dispatch path handled it. The BARE-PATH form (`K.len`,
/// resolved in `field_or_len`) hit "`len` is not a field or `.len` of data"; the
/// CALL form (`K.len()`, resolved in `eval_builtin`) hit "`len` is not defined on
/// data". Both were extended to return `DataBuf::size`. Pinned here in the
/// EXHIBIT shape: `const K = embed(...)` then `K.len` as a comptime int.
///
/// (Note recorded for R7m.7: the `data Kick = embed(...)` form does NOT expose a
/// readable `.len` — `data_value_readable` gates the value-read to StructLit
/// initializers only, so an embed data item's value is not readable as a field
/// receiver. The exhibit therefore binds embedded blobs to CONSTs to read their
/// comptime length. This is a scoping detail of the existing D-PP.5 receiver
/// gate, not a bankid/len defect.)
#[test]
fn embed_len_is_comptime_byte_length() {
    let src = "module m\n\
               const K = embed(\"embed_fixture.bin\")\n\
               data L: u8 = K.len\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (buf, _asserts, diags) = eval_data_with_root(&file, "L", None, Some(&vectors_dir()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    // 12 bytes in the fixture → a single u8 scalar cell of value 12.
    assert_eq!(buf.size, 1, "one u8 cell");
    assert_eq!(
        buf.cells,
        vec![sigil_frontend_emp::value::Cell::Scalar { value: 12, width: 1, signed: false, le: false }],
        "K.len must fold to 12",
    );
}

/// A sliced embed's `.len` reflects the slice length, not the file's — proving
/// `.len` reads the buffer's running byte size, not a file stat. Uses the CALL
/// form `K.len()` to also exercise the `eval_builtin` dispatch path.
#[test]
fn embed_slice_len_is_slice_length() {
    let src = "module m\n\
               const K = embed(\"embed_fixture.bin\", skip: 2, len: 4)\n\
               data L: u8 = K.len()\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (buf, _asserts, diags) = eval_data_with_root(&file, "L", None, Some(&vectors_dir()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(
        buf.cells,
        vec![sigil_frontend_emp::value::Cell::Scalar { value: 4, width: 1, signed: false, le: false }],
    );
}

// ---- R7m.3 (a): bankid emitted into a data cell folds to the bank id ---------

/// (a) `bankid("L")` in a width-1 cell folds to `(addr & $7F8000) >> 15`. The
/// symbol name is captured from a `Value::Str` — winptr's second capture path
/// (R7m.3: "exactly the winptr argument contract" = FnRef or Str). `L` is the
/// item's OWN label, placed at vma $8000 (bank 1), so the bank id is NONZERO:
/// `$8000 & $7F8000 = $8000`; `$8000 >> 15 = 1`. Byte-asserted through the full
/// resolve_layout+link path (68k section).
#[test]
fn bankid_width1_folds_to_bank_id_68k() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 data L: u8 = bankid(\"L\")\n\
               }\n";
    let m = lower_ok(src);
    let bytes = linked_bytes(&m, "s");
    assert_eq!(bytes[0], 0x01, "bankid at $8000 must fold to 1; got {:#X}", bytes[0]);
}

/// bankid via the FnRef capture path (a bare name — winptr's first capture path)
/// captures the NAME identically to `winptr(sfx)` and builds the same residual
/// tree over `Sym("sfx")`. Proven at the frontend cell (a `comptime fn` is erased
/// and has no link address, so this does not fold — the Str form's fold tests
/// prove folding end-to-end; this proves the FnRef arm reaches the SAME builtin).
#[test]
fn bankid_of_fn_ref_captures_name() {
    use sigil_frontend_emp::value::Cell;
    use sigil_ir::expr::{BinOp, Expr};
    let src = "module m\n\
               comptime fn sfx() -> u8 { 0 }\n\
               data B: u8 = bankid(sfx)\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (buf, _asserts, diags) = eval_data_with_root(&file, "B", None, None);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let expected = Expr::Binary {
        op: BinOp::Shr,
        lhs: Box::new(Expr::Binary {
            op: BinOp::And,
            lhs: Box::new(Expr::Sym("sfx".into())),
            rhs: Box::new(Expr::Int(0x7F8000)),
        }),
        rhs: Box::new(Expr::Int(15)),
    };
    assert_eq!(
        buf.expect("data buf").cells,
        vec![Cell::Expr { expr: expected, width: 1, le: false }],
        "bankid(sfx) must build (sfx & $7F8000) >> 15 over Sym(\"sfx\")",
    );
}

/// A label in bank 0 (vma < $8000) folds to 0 — the mask picks up no bank bits.
#[test]
fn bankid_bank_zero_folds_to_zero() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $0400) {\n\
                 data L: u8 = bankid(\"L\")\n\
               }\n";
    let m = lower_ok(src);
    let bytes = linked_bytes(&m, "s");
    assert_eq!(bytes[0], 0x00, "bankid at $0400 must fold to 0; got {:#X}", bytes[0]);
}

// ---- R7m.3 (f) / R7m.5: the Z80 end-to-end probe -----------------------------

/// (f) THE T3-review carry-forward: a width-2 `bankid(L)` cell inside a
/// `(cpu: z80)` section, compiled through resolve_layout+link, byte-asserted
/// LITTLE-ENDIAN (Value16Le). Z80 had NO LinkExpr source until bankid (no jbra),
/// so this is the first end-to-end proof that a Z80 `Cell::Expr` folds and writes
/// little-endian — closing the seam T3 could only cover in two half-unit tests
/// and discharging R7m.5's probe clause. `L` at vma $8000 → bank 1 → the word $0001
/// → little-endian bytes [01, 00].
#[test]
fn bankid_width2_z80_folds_little_endian() {
    let src = "module m\n\
               section s (cpu: z80, vma: $8000) {\n\
                 data L: u16 = bankid(\"L\")\n\
               }\n";
    let m = lower_ok(src);
    let bytes = linked_bytes(&m, "s");
    assert_eq!(&bytes[0..2], &[0x01, 0x00], "bankid word must be $0001 LE; got {:02X?}", &bytes[0..2]);
}

// ---- R7m.3 (b)/(c): ensure(bankid(A) == bankid(B)) defers to link ------------

/// (b) `ensure(bankid(A) == bankid(B), …)` with A and B in DIFFERENT banks fails
/// AT LINK with the guard's message. The comparison is a `LinkExpr` (lifted from
/// two bankid trees), so the guard defers to a `LinkAssert` with ZERO new code
/// (D-H.4). A at $0400 (bank 0), B at $8000 (bank 1) → the condition folds false.
#[test]
fn ensure_bankid_mismatch_fails_at_link() {
    let src = "module m\n\
               section a (cpu: m68000, vma: $0400) {\n\
                 data A: u8 = 0\n\
               }\n\
               section b (cpu: m68000, vma: $8000) {\n\
                 data B: u8 = 0\n\
                 ensure(bankid(\"A\") == bankid(\"B\"), \"samples must share a bank\")\n\
               }\n";
    let m = lower_ok(src);
    assert_eq!(m.link_asserts.len(), 1, "the bankid ensure must defer to one LinkAssert");
    let ds = link_assert_diags(&m);
    assert!(
        ds.iter().any(|d| d.message == "samples must share a bank"),
        "expected the guard message at link, got: {ds:?}"
    );
}

/// (c) The same-bank twin passes SILENTLY (zero diagnostics): A and B both in
/// bank 1 (both above $8000) → the condition folds true, no diagnostic.
#[test]
fn ensure_bankid_same_bank_passes_silently() {
    let src = "module m\n\
               section a (cpu: m68000, vma: $8000) {\n\
                 data A: u8 = 0\n\
               }\n\
               section b (cpu: m68000, vma: $8100) {\n\
                 data B: u8 = 0\n\
                 ensure(bankid(\"A\") == bankid(\"B\"), \"samples must share a bank\")\n\
               }\n";
    let m = lower_ok(src);
    assert_eq!(m.link_asserts.len(), 1, "the bankid ensure still defers");
    let ds = link_assert_diags(&m);
    assert!(ds.is_empty(), "same-bank guard must pass silently, got: {ds:?}");
}

// ---- R7m.3 (d): bankid in a comptime-required position refuses ---------------

/// (d) `bankid` steering a comptime array length refuses via the existing
/// reject_if_provisional choke point, with the `[bank.provisional]` steering
/// message (R7m.3) — NOT the `[here.provisional]` branch-sizing text (which does
/// not apply; bankid is link-time by construction).
#[test]
fn bankid_as_array_length_refuses_with_bank_message() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 data L: u8 = 0\n\
                 data Bad: [u8; bankid(\"L\")] = []\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        diags.iter().any(|d| d.message.contains("[bank.provisional]")
            && d.message.contains("emit it into a data cell or guard it with ensure")),
        "expected the [bank.provisional] steering message, got: {diags:?}"
    );
    // And NOT the here() branch-sizing text (provenance steering, R7m.3).
    assert!(
        !diags.iter().any(|d| d.message.contains("[here.provisional]")),
        "bankid must not surface the here() message, got: {diags:?}"
    );
}

// ---- R7m.3 (e): argument-form errors mirror winptr's -------------------------

/// (e) Wrong arity is diagnosed exactly like `winptr` (arity + "got N").
#[test]
fn bankid_wrong_arity_is_diagnosed() {
    let src = "module m\ndata X: u8 = bankid(A, B)\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        diags.iter().any(|d| d.message == "`bankid` expects exactly 1 argument, got 2"),
        "expected the arity diagnostic, got: {diags:?}"
    );
}

/// (e) A non-label argument (an integer) is diagnosed like `winptr`'s
/// "needs a symbol reference" error.
#[test]
fn bankid_non_symbol_argument_is_diagnosed() {
    let src = "module m\ndata X: u8 = bankid(5)\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        diags.iter().any(|d| d.message.contains("`bankid` needs a symbol reference")),
        "expected the symbol-reference diagnostic, got: {diags:?}"
    );
}

#[test]
fn unknown_attr_diagnostics_unchanged_alongside_bank() {
    // Unknown-attribute diagnostics (naming the offending attr) must still fire
    // exactly as before, even in a section that also carries a `bank:`.
    let src = "module m\n\
               section s (bank: $8000, bogus: 1) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        diags.iter().any(|d| d.message.contains("unknown attribute `bogus`")),
        "expected an unknown-attribute diagnostic, got: {diags:?}"
    );
}
