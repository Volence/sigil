//! T0 — prove the Core IR seam end-to-end with the smallest possible slice:
//! a `.emp` `data` item lowers to a `Module` whose linked image round-trips to
//! bytes, and a pointer field lands an `Abs32Be` fixup targeting the symbol.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::expr::BinOp;
use sigil_ir::{Expr, Fixup, FixupKind, Fragment, Section, SymbolTable};
use std::path::{Path, PathBuf};

/// The masked fixup target a `winptr(sym)` lowers to: `(sym & 0x7FFF) | 0x8000`
/// (AS `sfx_winptr`). Used by the windowed-pointer fixup-shape assertions below.
fn winptr_target(name: &str) -> Expr {
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
fn roundtrip_bytes() {
    let (file, perrs) = parse_str("module m\ndata X: [u8; 3] = [1, 2, 3]\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");

    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let bytes = sigil_link::flatten(&linked, 0x00);
    assert_eq!(bytes, vec![0x01, 0x02, 0x03]);
}

#[test]
fn multibyte_scalar_is_big_endian() {
    // The seam's whole point: a width>1 scalar must serialize big-endian
    // (M68000 order). `u16 = $1234` → [0x12, 0x34].
    let (file, perrs) = parse_str("module m\ndata W: u16 = $1234\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");

    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let bytes = sigil_link::flatten(&linked, 0x00);
    assert_eq!(bytes, vec![0x12, 0x34]);
}

#[test]
fn symref_makes_abs32_fixup() {
    let src = "module m\n\
               comptime fn init() -> u8 { 0 }\n\
               struct Obj { code: *u8, flags: u8 }\n\
               data D: Obj = Obj{ code: init, flags: 3 }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, _diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });

    // The point is the fixup SHAPE, not its resolution: an Abs32Be fixup at
    // offset 0 of the data fragment, targeting the symbol `init`.
    let section = module.sections.first().expect("one section");
    let fixups: Vec<&Fixup> = section
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(&d.fixups),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(
        fixups,
        vec![&Fixup {
            kind: FixupKind::Abs32Be,
            offset: 0,
            target: Expr::Sym("init".into()),
        }]
    );
}

// ---- T2: per-CPU byte order and the SymRef → Fixup table ----------------

/// Collect all fixups across a module's first section's data fragments.
fn section_fixups(module: &sigil_ir::Module) -> Vec<Fixup> {
    module
        .sections
        .first()
        .expect("one section")
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(d.fixups.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Concatenate the raw (pre-link) bytes of a module's first section's data
/// fragments. Used when a fixup targets an undefined external symbol, so the
/// module cannot be linked but its serialized image (holes zero-filled) still
/// byte-diffs.
fn raw_data_bytes(module: &sigil_ir::Module) -> Vec<u8> {
    module
        .sections
        .first()
        .expect("one section")
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(d.bytes.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Link a module to its flat image bytes (mirrors T0's link pattern).
fn linked_bytes(module: &sigil_ir::Module) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

#[test]
fn scalar_byte_order_per_cpu() {
    // The seam's whole point: the SAME `DataBuf` serializes big-endian on a
    // 68000 section and little-endian on a Z80 section (§4.5 / §7.2).
    // `u16 = 258` (0x0102) → [0x01, 0x02] on 68k, [0x02, 0x01] on Z80.
    let (file, perrs) = parse_str("module m\ndata W: u16 = 258\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (be_mod, be_diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(be_diags.is_empty(), "unexpected 68k diagnostics: {be_diags:?}");
    assert_eq!(linked_bytes(&be_mod), vec![0x01, 0x02]);

    let (le_mod, le_diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::Z80, include_root: None, defines: vec![] });
    assert!(le_diags.is_empty(), "unexpected Z80 diagnostics: {le_diags:?}");
    assert_eq!(linked_bytes(&le_mod), vec![0x02, 0x01]);
}

#[test]
fn symref_width4_68k_is_abs32be() {
    // A width-4 pointer in a 68000 section is an Abs32Be fixup (D-P4.5 row 1).
    let src = "module m\n\
               comptime fn init() -> u8 { 0 }\n\
               struct Obj { code: *u8, flags: u8 }\n\
               data D: Obj = Obj{ code: init, flags: 3 }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("init".into()) }]
    );
}

#[test]
fn winptr_in_z80_is_value16le() {
    // `winptr(sym): u16` in a Z80 section (R-T0.5): now a general link-expr VALUE
    // cell → a `Value16Le` fixup at offset 0 targeting the WINDOW-MASKED tree
    // `(sfx & 0x7FFF) | 0x8000` (the SAME target the old `BankPtr16Le` carried).
    // The folded value stays in [$8000,$FFFF], so its bytes are IDENTICAL to the
    // pre-R-T0.5 BankPtr16Le path (proven in lower_sections.rs).
    let src = "module m\n\
               comptime fn sfx() -> u8 { 0 }\n\
               data P: u16 = winptr(sfx)\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::Z80, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::Value16Le, offset: 0, target: winptr_target("sfx") }]
    );
    // The 2-byte hole is present in the image (zero-filled before linking).
    let section = module.sections.first().expect("one section");
    let bytes: usize = section
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(d.bytes.len()),
            _ => None,
        })
        .sum();
    assert_eq!(bytes, 2, "winptr reserves a 2-byte window pointer");
}

#[test]
fn winptr_in_68k_is_value16be() {
    // A `winptr(sym): u16` in a 68000 section (R-T0.5): a general link-expr VALUE
    // cell → `Value16Be`, targeting the same masked tree. The big-endian
    // counterpart of `winptr_in_z80_is_value16le`; bytes unchanged from the old
    // `BankPtr16Be` path.
    let src = "module m\n\
               comptime fn sfx() -> u8 { 0 }\n\
               data P: u16 = winptr(sfx)\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::Value16Be, offset: 0, target: winptr_target("sfx") }]
    );
    // The 2-byte hole is present in the image (zero-filled before linking).
    assert_eq!(raw_data_bytes(&module), vec![0x00, 0x00]);
}

#[test]
fn unwindowed_pointer_in_z80_is_error() {
    // A plain (un-windowed, width-4) pointer in a Z80 section is the
    // [cross-cpu.unwindowed-pointer] error naming the symbol (§7.2 / D-P4.5).
    let src = "module m\n\
               comptime fn init() -> u8 { 0 }\n\
               struct Obj { code: *u8, flags: u8 }\n\
               data D: Obj = Obj{ code: init, flags: 3 }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::Z80, include_root: None, defines: vec![] });
    assert!(
        diags.iter().any(|d| d.message.contains("[cross-cpu.unwindowed-pointer]")
            && d.message.contains("init")),
        "expected an unwindowed-pointer diagnostic naming `init`, got: {diags:?}"
    );
}

#[test]
fn mixed_table_byte_diff_68k() {
    // A small mixed `data` table byte-diffed against a hand-computed reference
    // (§8.3): a u16, a pointer (kept at an even offset to avoid the odd-field
    // layout warning), then a u8. On 68k: u16 big-endian, then a 4-byte Abs32
    // hole (zero-filled pre-link), then the u8 → 7 bytes total.
    let src = "module m\n\
               comptime fn init() -> u8 { 0 }\n\
               struct Row { tag: u16, code: *u8, flag: u8 }\n\
               data R: Row = Row{ tag: $1234, code: init, flag: $7F }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    // tag=0x1234 (BE) | code=0x00000000 (Abs32 hole) | flag=0x7F. The pointer
    // targets an external `init`, so byte-diff the raw pre-link image.
    assert_eq!(raw_data_bytes(&module), vec![0x12, 0x34, 0x00, 0x00, 0x00, 0x00, 0x7F]);
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::Abs32Be, offset: 2, target: Expr::Sym("init".into()) }]
    );
}

// ---- lexical gaps (Task 2): the `dc.w -1` signed-sentinel convention ----
//
// Real AS/Sonic assembly writes `dc.w -1` / `dc.b -1` as a sentinel or
// terminator: the two's-complement bit pattern `$FFFF` / `$FF`. In `.emp` the
// author reaches that bit pattern by writing a NEGATIVE VALUE of a SIGNED
// type (`i16`/`i8`), never by loosening an unsigned type. The convention,
// locked byte-for-byte below:
//
//   * Signed *values* (including sentinels) use `i8`/`i16` (or wider). A
//     literal `-1` in an `i16` slot lowers, through the ordinary emit path
//     (`sigil_frontend_emp::eval::emit`'s `[emit.out-of-range]` check, which
//     signed types pass since -1 is in `-32768..=32767`), to the two's-
//     complement image bytes `$FF $FF` on this big-endian (M68000) target —
//     `encode_scalar` (`lower/data.rs`) gets there by taking `(-1i128)
//     .to_be_bytes()`'s low `width` bytes, which are already all-`$FF` for
//     any negative one regardless of width. No new code: this is exercised
//     through the *unmodified* T2 byte-order seam covered elsewhere in this
//     file (`multibyte_scalar_is_big_endian`, `scalar_byte_order_per_cpu`).
//   * `u8`/`u16` are NOT loosened to accept negatives. `data T: u16 = -1`
//     is refused at emission time with `[emit.out-of-range]`, the same
//     totality guard that refuses `300` for a `u8` (see `eval_data.rs`'s
//     `array_element_out_of_type_range_is_emit_error` /
//     `refined_out_of_range_diagnoses_at_emission`). This is deliberate:
//     unsigned types never silently truncate/wrap a negative into a large
//     positive bit pattern — if you want the `$FFFF` bit pattern, say so with
//     a signed type, not by feeding `-1` to `u16`.
//   * A counted/terminated collection (e.g. a future `sentinel`-terminated
//     list builtin) owns its own terminator cell internally so *authors*
//     never hand-write a raw `-1` sentinel themselves; this test only locks
//     the underlying scalar encoding such a builtin would rely on.

#[test]
fn i16_neg1_lowers_to_ffff_signed_sentinel_bytes() {
    // `dc.w -1` ($FFFF) via `i16`. Bytes, not the structured `Cell`, are the
    // point — this is the lowest byte-producing layer reachable from a test
    // (`lower_module` → `sigil_link::flatten`), matching `roundtrip_bytes`/
    // `multibyte_scalar_is_big_endian` above.
    let (file, perrs) = parse_str("module m\ndata T: [i16; 1] = [-1]\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    assert_eq!(linked_bytes(&module), vec![0xFF, 0xFF]);
}

#[test]
fn i8_neg1_lowers_to_ff_signed_sentinel_byte() {
    // `dc.b -1` ($FF) via `i8`.
    let (file, perrs) = parse_str("module m\ndata T: [i8; 1] = [-1]\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    assert_eq!(linked_bytes(&module), vec![0xFF]);
}

#[test]
fn i16_array_shows_the_sentinel_in_terminator_position() {
    // The realistic shape: a run of positive values followed by a `-1`
    // terminator, e.g. an AS `dc.w 10, 20, -1` table. Two positive `i16`s
    // big-endian, then the `$FFFF` sentinel.
    let (file, perrs) = parse_str("module m\ndata T: [i16; 3] = [10, 20, -1]\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    assert_eq!(linked_bytes(&module), vec![0x00, 0x0A, 0x00, 0x14, 0xFF, 0xFF]);
}

// ---- lexical gaps (Task 4): string literals against a byte-array type ---
//
// `data Msg: [u8;N] = "HELLO"` — a string literal used directly where the
// declared type is a fixed-size array of a 1-byte primitive (`u8`/`i8`).
// Same ASCII-only rule as `bytes(...)`, and the SAME exact-length check an
// ordinary array literal gets: the author sizes `N` to include any
// terminator themselves (no implicit trailing 0).

#[test]
fn string_against_matching_byte_array_type_emits_ascii_bytes() {
    let (file, perrs) = parse_str("module m\ndata Msg: [u8;5] = \"HELLO\"\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    assert_eq!(linked_bytes(&module), vec![0x48, 0x45, 0x4C, 0x4C, 0x4F]);
}

#[test]
fn string_shorter_than_declared_byte_array_is_length_mismatch_error() {
    // No implicit terminator: a 5-byte string against a 4-byte array is a
    // plain length mismatch, not a silent truncation.
    let (file, perrs) = parse_str("module m\ndata Msg: [u8;4] = \"HELLO\"\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(
        diags.iter().any(|d| d.message.contains("length mismatch")
            && d.message.contains('4')
            && d.message.contains('5')),
        "expected a length-mismatch diagnostic naming 4 and 5, got {diags:?}"
    );
}

#[test]
fn string_with_null_escape_sized_to_include_terminator_emits_it() {
    // The author sizes the array to include the `\0` terminator explicitly.
    let (file, perrs) = parse_str("module m\ndata Msg: [u8;6] = \"HELLO\\0\"\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    assert_eq!(linked_bytes(&module), vec![0x48, 0x45, 0x4C, 0x4C, 0x4F, 0x00]);
}

#[test]
fn string_against_non_byte_element_array_type_stays_an_error() {
    // A string is only special-cased for a 1-byte element type; `[u16;N]`
    // stays the ordinary "expected an array" mismatch.
    let (file, perrs) = parse_str("module m\ndata Msg: [u16;3] = \"HELLO\"\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(
        diags.iter().any(|d| d.message.contains("expected an array") && d.message.contains("string")),
        "expected an 'expected an array ... got string' diagnostic, got {diags:?}"
    );
}

#[test]
fn string_against_byte_array_rejects_non_ascii() {
    let src = "module m\ndata Msg: [u8;1] = \"\u{e9}\"\n"; // "é"
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(
        diags.iter().any(|d| d.message.to_lowercase().contains("ascii")),
        "expected an ASCII-only diagnostic, got {diags:?}"
    );
}

// ---- lexical gaps (Task 4): pointer/symbol-ref non-collision ------------
//
// A string in a POINTER-typed field context must stay a SYMBOL NAME
// (`Cell::SymRef` via `lower_ptr`, unchanged) — the byte-emission path added
// above must never touch `Ty::Ptr`. Proven with the SAME fixup-shape
// assertion `symref_makes_abs32_fixup` uses, but the pointer field is
// initialized with a plain string literal instead of a bare fn name.
#[test]
fn string_in_pointer_field_is_still_a_symbol_ref_not_bytes() {
    let src = "module m\n\
               struct Obj { code: *u8, flags: u8 }\n\
               data D: Obj = Obj{ code: \"some_symbol\", flags: 3 }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("some_symbol".into()) }]
    );
}

#[test]
fn u16_neg1_is_rejected_not_wrapped() {
    // The guardrail half of the convention: `u16` does NOT accept `-1` and
    // silently wrap it to `$FFFF`. It is an `[emit.out-of-range]` error, the
    // same totality check `eval_data.rs`'s
    // `refined_out_of_range_diagnoses_at_emission` exercises for `300` on a
    // `u8` — this is that same check's negative-value edge, named here
    // because it's the reason the convention says "use `i16`/`i8`", not
    // "loosen `u16`/`u8`".
    let (file, perrs) = parse_str("module m\ndata T: [u16; 1] = [-1]\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(
        diags.iter().any(|d| d.message.contains("[emit.out-of-range]") && d.message.contains("-1")),
        "expected an [emit.out-of-range] refusing -1 for u16, got {diags:?}"
    );
}

// ---- Task 1 (sound-migration T0): `u16le` data cells (R-T0.1, DSM.7) ----
//
// An explicit little-endian 16-bit type keyword, usable from ANY section
// (the point: a 68k-side section emitting Z80-consumed bytes). NOT CPU
// inference — the `le` flag always wins over the section's CPU. No `u32le`,
// no other endian variant (YAGNI until a customer exists).

#[test]
fn u16le_scalar_in_68k_section_emits_little_endian() {
    // `data X: u16le = $1234` in a (cpu: m68000) section → bytes 34 12 (the
    // opposite order from `multibyte_scalar_is_big_endian`'s plain `u16`).
    let (file, perrs) = parse_str("module m\ndata X: u16le = $1234\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    assert_eq!(linked_bytes(&module), vec![0x34, 0x12]);
}

#[test]
fn u16le_equals_u16_on_z80() {
    // Z80 sections are already little-endian: `u16le` must emit EXACTLY what
    // plain `u16` emits there — no double byte-swap.
    let (le_file, perrs) = parse_str("module m\ndata X: u16le = $1234\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (le_mod, le_diags) = lower_module(&le_file, &LowerOptions { initial_cpu: Cpu::Z80, include_root: None, defines: vec![] });
    assert!(le_diags.is_empty(), "unexpected lowering diagnostics: {le_diags:?}");

    let (be_file, perrs) = parse_str("module m\ndata X: u16 = $1234\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (be_mod, be_diags) = lower_module(&be_file, &LowerOptions { initial_cpu: Cpu::Z80, include_root: None, defines: vec![] });
    assert!(be_diags.is_empty(), "unexpected lowering diagnostics: {be_diags:?}");

    assert_eq!(linked_bytes(&le_mod), linked_bytes(&be_mod));
    assert_eq!(linked_bytes(&le_mod), vec![0x34, 0x12]);
}

#[test]
fn u16le_linkexpr_cell_uses_value16le_on_68k() {
    // A link-expr value (`bankid("L")`) landing in a `u16le` cell must select
    // `FixupKind::Value16Le` even in a 68k (normally big-endian) section — the
    // `le` flag overrides the section CPU for fixup-kind selection too, not
    // just the plain-scalar path.
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 data L: u16le = bankid(\"L\")\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    assert_eq!(
        section_fixups(&module),
        vec![Fixup {
            kind: FixupKind::Value16Le,
            offset: 0,
            target: Expr::Binary {
                op: BinOp::Shr,
                lhs: Box::new(Expr::Binary {
                    op: BinOp::And,
                    lhs: Box::new(Expr::Sym("L".into())),
                    rhs: Box::new(Expr::Int(0x7F8000)),
                }),
                rhs: Box::new(Expr::Int(15)),
            },
        }]
    );
    // `L` is placed at vma $8000 (bank 1): the fold value is 1. Value16Le
    // writes it little-endian: [01, 00].
    assert_eq!(linked_bytes(&module), vec![0x01, 0x00]);
}

#[test]
fn u16le_range_rules_are_identical_to_u16() {
    // The `le` flag never affects the accepted range — still 0..=$FFFF, so an
    // out-of-range value errors exactly like plain `u16` (see
    // `u16_neg1_is_rejected_not_wrapped` above): `-1` is refused, not wrapped.
    let (file, perrs) = parse_str("module m\ndata T: [u16le; 1] = [-1]\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(
        diags.iter().any(|d| d.message.contains("[emit.out-of-range]") && d.message.contains("-1")),
        "expected an [emit.out-of-range] refusing -1 for u16le, got {diags:?}"
    );
}

// ---- sound-migration T2 Task 1: comptime `defines` (-D) ------------------
//
// A `.emp` module can be lowered under different comptime `defines` — the
// analogue of AS's `-D __DEBUG__` — so ONE module source produces different
// build shapes. Frozen ruling R1: each `(name, value)` pair is injected as a
// resolved comptime int const into the module's global scope BEFORE any item
// evaluates; a module-declared item sharing a define's name is a hard
// `[defines.collision]` error, never a silent shadow either direction.

#[test]
fn defines_are_visible_as_comptime_consts() {
    let src = "module t\n\
               const N = if DEBUG == 1 { 3 } else { 1 }\n\
               data Tbl: [u8; N] = if DEBUG == 1 { [1, 2, 3] } else { [7] }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![("DEBUG".into(), 1)] },
    );
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags:?}");
    assert_eq!(linked_bytes(&module), vec![1, 2, 3]);

    let (module0, diags0) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![("DEBUG".into(), 0)] },
    );
    assert!(diags0.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags0:?}");
    assert_eq!(linked_bytes(&module0), vec![7]);
}

#[test]
fn define_colliding_with_module_decl_errors() {
    let src = "module t\nconst DEBUG = 0\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (_module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![("DEBUG".into(), 1)] },
    );
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error && d.message.contains("[defines.collision]")),
        "expected a [defines.collision] error, got {diags:?}"
    );
}

#[test]
fn define_colliding_with_proc_name_errors() {
    // R1 says a module-declared ITEM of the define's name is a hard error —
    // and a proc is an item too (spec-review catch: without the Proc arm in
    // `validate_defines`, `-D Foo=5` + `proc Foo` compiled silently and a
    // `data P: u32 = Foo` initializer read the DEFINE's int instead of the
    // proc's label, the exact silent-shadow R1 forbids).
    let src = "module t\nproc Foo() {\n    rts\n}\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (_module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![("Foo".into(), 5)] },
    );
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error && d.message.contains("[defines.collision]")),
        "expected a [defines.collision] error for a proc-name collision, got {diags:?}"
    );
}

// ---- sound-migration T2 Task 2: MT-shape capability probes (P1-P4) ------
//
// `mt_bank.emp` (Task 5) needs five constructs no existing test exercises
// end-to-end together: a conditional `embed(...) / Data.empty` data item, a
// `[*u8; N]` pointer-array of string elements, an `if`-expression driving
// both a `const` array LENGTH and the array VALUE, a length-mismatch on that
// shape, and an `ensure` mixing a comptime `.len` with a pinned int const.
// Each probe is written to PIN bytes/offsets/addresses, not merely "no error".

/// The fixture directory `embed` resolves paths against for these probes:
/// the SAME `tests/vectors/` fixture `sandbox_embed.rs`/`sandbox_hermeticity.rs`
/// use, containing the deterministic `embed_fixture.bin` (12 bytes, `0x00..=0x0B`).
/// This IS the established "include_root tempdir-adjacent fixture" pattern for
/// `embed` in this crate — a real ad-hoc `tempfile::tempdir()` would just
/// reconstruct this same fixture at test time for no added coverage.
fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("vectors")
}

const EMBED_FIXTURE_BYTES: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

fn section<'a>(module: &'a sigil_ir::Module, name: &str) -> &'a Section {
    module
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section `{name}`"))
}

fn label_offset(sec: &Section, name: &str) -> u32 {
    sec.labels.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("no label `{name}`")).offset
}

/// Lower `src` with the given `defines`, resolving `embed(...)` against
/// [`vectors_dir`]. Asserts a clean parse; returns `(module, diagnostics)`.
fn lower_with_defines_and_root(
    src: &str,
    defines: Vec<(String, i128)>,
) -> (sigil_ir::Module, Vec<sigil_span::Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: Some(vectors_dir()), defines },
    )
}

// ---- P1: `if C { embed(...) } else { Data.empty }` -----------------------

#[test]
fn p1_conditional_embed_true_arm_emits_real_bytes() {
    let src = "module m\n\
               section s (vma: $8000) {\n\
               data X = if C == 1 { embed(\"embed_fixture.bin\") } else { Data.empty }\n\
               data Next: u8 = $AA\n\
               }\n";
    let (module, diags) = lower_with_defines_and_root(src, vec![("C".into(), 1)]);
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags:?}");

    let sec = section(&module, "s");
    let x_off = label_offset(sec, "X");
    let next_off = label_offset(sec, "Next");
    assert_eq!(x_off, 0, "X is the section's first item");
    assert_eq!(
        next_off, 12,
        "Next must land right after X's 12 embedded bytes (no padding, no gap)"
    );

    let bytes = linked_bytes(&module);
    assert_eq!(&bytes[0..12], &EMBED_FIXTURE_BYTES, "the true arm embeds the real file bytes");
    assert_eq!(bytes[12], 0xAA, "Next's byte follows immediately");
    assert_eq!(bytes.len(), 13, "no extra padding bytes anywhere");
}

#[test]
fn p1_conditional_embed_false_arm_is_zero_length_but_labeled() {
    // The else arm (`Data.empty`) must produce a ZERO-length data item: the
    // label `X` is still defined, it emits zero bytes, and `Next` — the NEXT
    // data item in the SAME section — lands at the SAME offset X would have
    // occupied had X emitted nothing (i.e. offset 0, since X is first).
    let src = "module m\n\
               section s (vma: $8000) {\n\
               data X = if C == 1 { embed(\"embed_fixture.bin\") } else { Data.empty }\n\
               data Next: u8 = $AA\n\
               }\n";
    let (module, diags) = lower_with_defines_and_root(src, vec![("C".into(), 0)]);
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags:?}");

    let sec = section(&module, "s");
    let x_off = label_offset(sec, "X");
    let next_off = label_offset(sec, "Next");
    assert_eq!(x_off, 0, "X's label is still defined");
    assert_eq!(next_off, 0, "Next lands at X's offset — X emitted zero bytes");
    assert_eq!(x_off, next_off, "the following item lands at the SAME offset as the empty item");

    let bytes = linked_bytes(&module);
    assert_eq!(bytes, vec![0xAA], "only Next's single byte — X contributed nothing");
}

// ---- P2: `[*u8; N]` pointer array with string elements -------------------

#[test]
fn p2_pointer_array_of_three_strings_emits_three_abs32_symrefs() {
    let src = "module m\n\
               data A: [u8;1] = [1]\n\
               data B: [u8;1] = [2]\n\
               data C: [u8;1] = [3]\n\
               data T: [*u8; 3] = [\"A\", \"B\", \"C\"]\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags:?}");

    // Three 4-byte Abs32Be fixups at offsets 0, 4, 8, targeting A/B/C.
    assert_eq!(
        section_fixups(&module),
        vec![
            Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("A".into()) },
            Fixup { kind: FixupKind::Abs32Be, offset: 4, target: Expr::Sym("B".into()) },
            Fixup { kind: FixupKind::Abs32Be, offset: 8, target: Expr::Sym("C".into()) },
        ]
    );

    // Resolve to the labels' actual linked addresses (A=0, B=1, C=2 in the
    // default `text` section: A/B/C are each 1 byte, then T's 12-byte table).
    let bytes = linked_bytes(&module);
    assert_eq!(bytes.len(), 3 + 12, "3 one-byte blobs + a 12-byte pointer table");
    assert_eq!(&bytes[3..7], &[0x00, 0x00, 0x00, 0x00], "T[0] -> A @ address 0");
    assert_eq!(&bytes[7..11], &[0x00, 0x00, 0x00, 0x01], "T[1] -> B @ address 1");
    assert_eq!(&bytes[11..15], &[0x00, 0x00, 0x00, 0x02], "T[2] -> C @ address 2");
}

#[test]
fn p2_pointer_array_of_one_string_emits_one_abs32_symref() {
    let src = "module m\n\
               data A: [u8;1] = [1]\n\
               data T: [*u8; 1] = [\"A\"]\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags:?}");

    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("A".into()) }]
    );
    let bytes = linked_bytes(&module);
    assert_eq!(bytes.len(), 1 + 4, "the one-byte blob + a 4-byte pointer cell");
    assert_eq!(&bytes[1..5], &[0x00, 0x00, 0x00, 0x00], "T[0] -> A @ address 0");
}

// ---- P3: `if`-expression driving BOTH a const array length and the value -

#[test]
fn p3_if_expression_const_length_drives_matching_array_shape_debug_1() {
    let src = "module m\n\
               data A: [u8;1] = [1]\n\
               data B: [u8;1] = [2]\n\
               data C: [u8;1] = [3]\n\
               const N = if D == 1 { 3 } else { 1 }\n\
               data T: [*u8; N] = if D == 1 { [\"A\", \"B\", \"C\"] } else { [\"A\"] }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![("D".into(), 1)] },
    );
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags:?}");
    assert_eq!(
        section_fixups(&module),
        vec![
            Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("A".into()) },
            Fixup { kind: FixupKind::Abs32Be, offset: 4, target: Expr::Sym("B".into()) },
            Fixup { kind: FixupKind::Abs32Be, offset: 8, target: Expr::Sym("C".into()) },
        ],
        "D=1 selects the 3-element shape, N=3"
    );
}

#[test]
fn p3_if_expression_const_length_drives_matching_array_shape_debug_0() {
    let src = "module m\n\
               data A: [u8;1] = [1]\n\
               data B: [u8;1] = [2]\n\
               data C: [u8;1] = [3]\n\
               const N = if D == 1 { 3 } else { 1 }\n\
               data T: [*u8; N] = if D == 1 { [\"A\", \"B\", \"C\"] } else { [\"A\"] }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![("D".into(), 0)] },
    );
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags:?}");
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("A".into()) }],
        "D=0 selects the 1-element shape, N=1"
    );
}

#[test]
fn p3_mismatched_array_length_against_const_n_is_clean_error_not_panic() {
    // N=3 (D==1) but the value arm supplies only 2 elements — a clean
    // diagnostic naming the mismatch, never a panic.
    let src = "module m\n\
               data A: [u8;1] = [1]\n\
               data B: [u8;1] = [2]\n\
               const N = if D == 1 { 3 } else { 1 }\n\
               data T: [*u8; N] = [\"A\", \"B\"]\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![("D".into(), 1)] },
    );
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("length mismatch")
            && d.message.contains('3')
            && d.message.contains('2')),
        "expected a clean length-mismatch error naming 3 and 2, got {diags:?}"
    );
}

// ---- P4: `ensure` mixing a comptime embed `.len` and a pinned int const -

#[test]
fn p4_ensure_len_against_pinned_const_passes_when_equal() {
    let src = "module m\n\
               const Blob = embed(\"embed_fixture.bin\")\n\
               const PINNED = 12\n\
               ensure(Blob.len == PINNED, \"blob length drifted: want {PINNED}, got {Blob.len}\")\n\
               data D: [u8;1] = [1]\n";
    let (_module, diags) = lower_with_defines_and_root(src, vec![]);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "matching len must pass silently, got: {diags:?}"
    );
}

#[test]
fn p4_ensure_len_against_pinned_const_fires_loud_message_when_unequal() {
    let src = "module m\n\
               const Blob = embed(\"embed_fixture.bin\")\n\
               const PINNED = 999\n\
               ensure(Blob.len == PINNED, \"blob length drifted: want {PINNED}, got {Blob.len}\")\n\
               data D: [u8;1] = [1]\n";
    let (_module, diags) = lower_with_defines_and_root(src, vec![]);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("blob length drifted: want 999, got 12")),
        "expected the ensure's interpolated message naming 999 and 12, got: {diags:?}"
    );
}

// ---- sound-migration T3 Task 1: SFX-shape capability probes (P2, P3) ----
//
// `sfx_bank.emp` (T3 Task 3) needs two constructs no existing test exercises:
// an `embed(...)` of a ZERO-BYTE file (the two PSG-only patch banks
// `sfx_36_patches.bin`/`sfx_62_patches.bin` are empty), and a `[*u8; N]`
// pointer array carrying NULL (`0`) entries alongside SymRef strings (the
// sparse `SfxTable`: 9 syms + 126 `0` cells for the unused ids). T2's P1
// proved `Data.empty` (an emp-side zero-length item); T2's P2 proved a
// `[*u8; N]` of ONLY string elements. These close the remaining two gaps.

// ---- P2: `embed(...)` of a ZERO-BYTE file -------------------------------

#[test]
fn p2_zero_byte_embed_is_labeled_zero_length_next_item_same_offset() {
    // `data X = embed("empty.bin")` where empty.bin is a 0-byte file: X's label
    // is defined, X emits ZERO bytes, and the FOLLOWING item lands at the SAME
    // offset X occupies (the embed-of-empty-FILE path, distinct from T2 P1's
    // `Data.empty` builtin). Mirrors `p1_conditional_embed_false_arm_...`.
    let src = "module m\n\
               section s (vma: $8000) {\n\
               data X = embed(\"empty.bin\")\n\
               data Next: u8 = $AA\n\
               }\n";
    let (module, diags) = lower_with_defines_and_root(src, vec![]);
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags:?}");

    let sec = section(&module, "s");
    let x_off = label_offset(sec, "X");
    let next_off = label_offset(sec, "Next");
    assert_eq!(x_off, 0, "X's label is still defined at the section start");
    assert_eq!(next_off, 0, "Next lands at X's offset — the zero-byte embed emitted nothing");
    assert_eq!(x_off, next_off, "the following item lands at the SAME offset as the empty embed");

    let bytes = linked_bytes(&module);
    assert_eq!(bytes, vec![0xAA], "only Next's single byte — the empty embed contributed nothing");
}

// ---- P3: NULL (`0`) entries in a `[*u8; N]` pointer array ----------------

#[test]
fn p3_null_entries_in_pointer_array_lower_as_zero_cells_no_fixup() {
    // `data T: [*u8; 5] = ["A", 0, 0, "B", 0]` — the sparse SfxTable shape. The
    // string elements lower as Abs32Be SymRef fixups (at offsets 0 and 12); the
    // `0` elements lower as PLAIN zero cells with NO fixup. Linked bytes:
    // A's addr, 0, 0, B's addr, 0.
    let src = "module m\n\
               data A: [u8;1] = [1]\n\
               data B: [u8;1] = [2]\n\
               data T: [*u8; 5] = [\"A\", 0, 0, \"B\", 0]\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags:?}");

    // Exactly TWO fixups — the string elements only; the `0` cells carry none.
    assert_eq!(
        section_fixups(&module),
        vec![
            Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("A".into()) },
            Fixup { kind: FixupKind::Abs32Be, offset: 12, target: Expr::Sym("B".into()) },
        ],
        "only the sym entries (offsets 0, 12) get Abs32Be fixups; the `0` cells get none"
    );

    // A=0, B=1 (each 1 byte) then T's 5*4=20-byte table.
    let bytes = linked_bytes(&module);
    assert_eq!(bytes.len(), 2 + 20, "2 one-byte blobs + a 20-byte pointer table");
    assert_eq!(&bytes[2..6], &[0x00, 0x00, 0x00, 0x00], "T[0] -> A @ address 0");
    assert_eq!(&bytes[6..10], &[0x00, 0x00, 0x00, 0x00], "T[1] = null (0)");
    assert_eq!(&bytes[10..14], &[0x00, 0x00, 0x00, 0x00], "T[2] = null (0)");
    assert_eq!(&bytes[14..18], &[0x00, 0x00, 0x00, 0x01], "T[3] -> B @ address 1");
    assert_eq!(&bytes[18..22], &[0x00, 0x00, 0x00, 0x00], "T[4] = null (0)");
}

#[test]
fn p3_nonzero_int_entry_in_pointer_array_folds_to_absolute_cell() {
    // A non-zero int element in a `*u8` array folds to an absolute (Abs32Be
    // width-4) VALUE cell — pin whatever behavior the `0`-null path implements
    // so a stray nonzero int can't silently do something different. `$1234`
    // (the only int besides `0`) lands big-endian in its 4-byte cell.
    let src = "module m\n\
               data A: [u8;1] = [1]\n\
               data T: [*u8; 2] = [\"A\", $1234]\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "unexpected errors: {diags:?}");

    // Only the sym entry gets a fixup; the int is a folded absolute cell.
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("A".into()) }],
        "only the sym entry gets a fixup; the int is a folded absolute cell"
    );
    let bytes = linked_bytes(&module);
    assert_eq!(bytes.len(), 1 + 8, "the one-byte blob + a 2-entry (8-byte) pointer table");
    assert_eq!(&bytes[1..5], &[0x00, 0x00, 0x00, 0x00], "T[0] -> A @ address 0");
    assert_eq!(&bytes[5..9], &[0x00, 0x00, 0x12, 0x34], "T[1] = $1234 as a big-endian absolute cell");
}
