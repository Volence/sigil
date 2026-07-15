//! SST overlay data model (Spec 2, Plan 7 backlog #6, Part A — D6.A1/A2/A7/A9):
//! window resolution, field layout, always-on declaration checks, and
//! `offsetof`/`sizeof` support. Each case parses a full `.emp` file, lowers it
//! via the same `lower_module` entry the CLI uses, and asserts on the resulting
//! diagnostics / linked bytes.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

/// The base struct used throughout: a pitcher-plant-shaped SST with a
/// byte-array `sst_custom` window at `$2E` (34 bytes).
///
/// NOTE (deviation, see report): the design-doc SST spelling uses SPARSE
/// `@offset` fields (`x_pos: u16 @ $10`, …), but this codebase's struct layout
/// is DENSE — `@offset` is an ASSERTION over the packed layout, not a placement
/// with gaps (layout.rs `check_struct_offsets`; `eval_layout.rs` line 146
/// exemplar). So explicit `reserved` padding fields carry the field spacing,
/// and every field's `@offset` (and the `(size:)`) now matches the packed
/// layout exactly. `sst_custom` still genuinely lands at `$2E`, size 34, which
/// is all the overlay window resolution depends on.
const SST: &str = "struct Sst (size: $50) {\n    \
    id: u16,\n    \
    _pad0: [u8; 14] @ $2,\n    \
    x_pos: u16 @ $10,\n    \
    _pad1: [u8; 8] @ $12,\n    \
    y_vel: u16 @ $1A,\n    \
    _pad2: [u8; 18] @ $1C,\n    \
    sst_custom: [u8; 34] @ $2E,\n\
}\n";

/// Lower `src` (asserting a clean parse) and return `(module, diagnostic messages)`.
fn lower(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    (module, diags.into_iter().map(|d| d.message).collect())
}

fn msgs(src: &str) -> Vec<String> {
    lower(src).1
}

/// Lower `src`, returning `(module, ERROR-level diagnostic messages)`. Warnings
/// (notably the design-mandated `[layout.odd-field]` lint on an overlay word at
/// an odd offset — D6.A2) are filtered out, so a "clean overlay" case asserts on
/// errors alone.
fn lower_errors(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    let errs = diags
        .into_iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .map(|d| d.message)
        .collect();
    (module, errs)
}

/// Link the lowered module and return the bytes of its (single) default section.
fn linked_bytes(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    // The overlay tests each emit exactly one data item into the default section.
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

// ---- 1. clean overlay: offsetof / sizeof --------------------------------

#[test]
fn overlay_offsetof_and_sizeof_bytes() {
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8, charge: u16 }}\n\
         data d: [u8;2] = [offsetof(V, charge), sizeof(V)]\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    // `charge` follows `timer: u8` at overlay-relative offset 1; sizeof = 3.
    assert_eq!(linked_bytes(&module), vec![0x01, 0x03], "offsetof(V,charge)=1, sizeof(V)=3");
    // The odd-field lint applies to overlays too (D6.A2): `charge` is a u16 at
    // overlay-relative offset 1 (odd) — a design-mandated WARNING, not an error.
    let all = msgs(&src);
    assert!(
        all.iter().any(|m| m.contains("[layout.odd-field]") && m.contains("charge")),
        "want the overlay odd-field warning, got: {all:?}"
    );
}

// ---- 2. window overflow (always-on) -------------------------------------

#[test]
fn overlay_window_overflow_is_always_on() {
    // 35 bytes of fields into a 34-byte window — nothing accesses the overlay,
    // yet the capacity check must fire (D6.A2 always-on).
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ blob: [u8; 35] }}\n\
         data d: [u8;1] = [1]\n"
    );
    let d = msgs(&src);
    assert!(
        d.iter().any(|m| m.contains("[overlay.window-overflow]") && m.contains("over by 1")),
        "want [overlay.window-overflow] over by 1, got: {d:?}"
    );
}

// ---- 3. unknown window --------------------------------------------------

#[test]
fn overlay_unknown_window() {
    let src = format!("module m\n{SST}vars V: no_such_window {{ t: u8 }}\n");
    let d = msgs(&src);
    assert!(
        d.iter().any(|m| m.contains("[overlay.unknown-window]") && m.contains("no_such_window")),
        "want [overlay.unknown-window] naming no_such_window, got: {d:?}"
    );
}

// ---- 4. ambiguous window + dotted disambiguator -------------------------

#[test]
fn overlay_ambiguous_window() {
    // Two structs each expose a `[u8; N]` field named `scratch`.
    let src = "module m\n\
        struct Sst1 (size: 8) { scratch: [u8; 8] }\n\
        struct Sst2 (size: 8) { scratch: [u8; 8] }\n\
        vars V: scratch { t: u8 }\n";
    let d = msgs(src);
    assert!(
        d.iter().any(|m| {
            m.contains("[overlay.ambiguous-window]")
                && m.contains("Sst1.scratch")
                && m.contains("Sst2.scratch")
        }),
        "want [overlay.ambiguous-window] naming both candidates, got: {d:?}"
    );
}

#[test]
fn overlay_dotted_window_disambiguates() {
    let src = "module m\n\
        struct Sst1 (size: 8) { scratch: [u8; 8] }\n\
        struct Sst2 (size: 8) { scratch: [u8; 8] }\n\
        vars V: Sst2.scratch { t: u8 }\n\
        data d: [u8;1] = [sizeof(V)]\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "dotted window must resolve cleanly, got: {diags:?}");
    assert_eq!(linked_bytes(&module), vec![0x01], "sizeof(V)=1");
}

// ---- 5. shadows a direct base-struct field ------------------------------

#[test]
fn overlay_shadows_field() {
    let src = format!("module m\n{SST}vars V: sst_custom {{ x_pos: u8 }}\n");
    let d = msgs(&src);
    assert!(
        d.iter().any(|m| m.contains("[overlay.shadows-field]") && m.contains("x_pos")),
        "want [overlay.shadows-field] naming x_pos, got: {d:?}"
    );
}

// ---- 6. dotted window resolves identically to bare ----------------------

#[test]
fn overlay_dotted_window_matches_bare() {
    let src = format!(
        "module m\n{SST}vars V: Sst.sst_custom {{ timer: u8, charge: u16 }}\n\
         data d: [u8;2] = [offsetof(V, charge), sizeof(V)]\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    assert_eq!(linked_bytes(&module), vec![0x01, 0x03]);
}

// ---- odd-field lint keys on WINDOW-ABSOLUTE parity ----------------------
//
// The lint's job is runtime honesty: what matters is the field's in-memory
// offset within the base struct (`window_offset + overlay-relative offset`),
// not the overlay-relative offset alone. An odd window base flips the parity.

#[test]
fn overlay_odd_window_base_no_false_positive() {
    // `win` sits at struct offset 1 (ODD base). `w: u16` at overlay-relative
    // offset 1 lands at in-memory offset 1+1 = 2 — EVEN, so NO odd-field
    // warning may fire (keying on the relative offset alone would false-warn).
    let src = "module m\n\
        struct S (size: 9) { a: u8, win: [u8; 8] @ 1 }\n\
        vars V: win { t: u8, w: u16 }\n";
    let d = msgs(src);
    assert!(
        !d.iter().any(|m| m.contains("[layout.odd-field]") && m.contains("overlay V")),
        "u16 at even in-memory offset must not warn, got: {d:?}"
    );
}

#[test]
fn overlay_odd_window_base_catches_odd_memory_offset() {
    // Same odd window base; `w: u16` at overlay-relative offset 0 lands at
    // in-memory offset 1+0 = 1 — ODD, so the warning MUST fire (keying on the
    // relative offset alone would silently miss it).
    let src = "module m\n\
        struct S (size: 9) { a: u8, win: [u8; 8] @ 1 }\n\
        vars V: win { w: u16 }\n";
    let d = msgs(src);
    assert!(
        d.iter().any(|m| {
            m.contains("[layout.odd-field]") && m.contains("overlay V") && m.contains("field w")
        }),
        "u16 at odd in-memory offset must warn, got: {d:?}"
    );
}

// ---- decl errors report ONCE across passes -------------------------------

#[test]
fn overlay_error_reported_once_across_passes() {
    // An erroring overlay that is BOTH validated by the always-on `Item::Vars`
    // arm AND referenced via `sizeof` in a data item must report its decl error
    // exactly once — matching the struct exemplar, whose decl checks fire only
    // in the single forcing evaluator.
    let src = format!(
        "module m\n{SST}vars V: no_such_window {{ t: u8 }}\n\
         data d: [u8;1] = [sizeof(V)]\n"
    );
    let d = msgs(&src);
    let n = d.iter().filter(|m| m.contains("[overlay.unknown-window]")).count();
    assert_eq!(n, 1, "overlay decl error must report exactly once, got {n}: {d:?}");
}

// ---- 7. non-byte-array window is rejected (v1) --------------------------

#[test]
fn overlay_non_byte_array_window_rejected() {
    let src = "module m\n\
        struct S (size: 16) { words: [u16; 8] }\n\
        vars V: words { t: u8 }\n";
    let d = msgs(src);
    assert!(
        d.iter().any(|m| m.contains("[overlay.window-not-bytes]") && m.contains("words")),
        "want [overlay.window-not-bytes] for a non-[u8;N] field, got: {d:?}"
    );
}

#[test]
fn overlay_signed_byte_array_window_rejected() {
    // `[i8; N]` is not a `[u8; N]` window (D6.A1 v1: unsigned bytes only).
    let src = "module m\n\
        struct S (size: 8) { win: [i8; 8] }\n\
        vars V: win { t: u8 }\n";
    let d = msgs(src);
    assert!(
        d.iter().any(|m| m.contains("[overlay.window-not-bytes]") && m.contains("win")),
        "want [overlay.window-not-bytes] for an [i8;N] window, got: {d:?}"
    );
}

// ---- bare-window scan must not force unrelated struct layouts -----------

#[test]
fn bare_window_scan_does_not_validate_unrelated_structs() {
    // `Bad` is never referenced: its `[layout.odd-field]`-worthy layout (u16 at
    // offset 1) must stay unvalidated, exactly as it would with no overlay in
    // the module (struct decl checks fire only when a layout is FORCED). The
    // bare-window candidate scan must match by AST field name, not by laying
    // out every in-scope struct.
    let src = "module m\n\
        struct Bad (size: 3) { a: u8, w: u16 }\n\
        struct S (size: 9) { a: u8, win: [u8; 8] @ 1 }\n\
        vars V: win { t: u8 }\n";
    let d = msgs(src);
    assert!(
        !d.iter().any(|m| m.contains("[layout.odd-field] struct Bad")),
        "declaring a bare-window overlay must not validate unrelated structs, got: {d:?}"
    );
}

// ---- offsetof on an overlay: unknown field -------------------------------

#[test]
fn overlay_offsetof_unknown_field() {
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         data d: [u8;1] = [offsetof(V, no_such)]\n"
    );
    let d = msgs(&src);
    assert!(
        d.iter().any(|m| m.contains("offsetof") && m.contains("V") && m.contains("no_such")),
        "want the overlay unknown-field offsetof error, got: {d:?}"
    );
}

// ---- >2-segment window path ----------------------------------------------

#[test]
fn overlay_three_segment_window_path_rejected() {
    // The parser accepts `ident (. ident)*` unbounded, so a 3-segment path
    // reaches resolution and must get the bad-window error naming the form.
    let src = "module m\n\
        struct S (size: 8) { win: [u8; 8] }\n\
        vars V: a.b.c { t: u8 }\n";
    let d = msgs(src);
    assert!(
        d.iter().any(|m| m.contains("[overlay.bad-window]") && m.contains("a.b.c")),
        "want [overlay.bad-window] for a 3-segment window path, got: {d:?}"
    );
}

// =========================================================================
// Task 6 — field-access-as-displacement, bare form (D6.A3/A5/A6/A10).
//
// In a proc body `proc p (a0: *Sst) { … }`, a bare field name in the
// displacement position `f(a0)` resolves in FIELD SPACE (S's direct fields ∪
// in-scope overlays over S) to the comptime integer offset, taking the IDENTICAL
// `DispInd → Disp16An` path as an integer literal (byte-neutral, D6.A10).
// =========================================================================

/// Full linked byte image of a lowered module's (single) default section — the
/// proc-body form of [`linked_bytes`]. Procs emit into the default section too.
fn proc_bytes(m: &Module) -> Vec<u8> {
    linked_bytes(m)
}

// ---- 1. headline: overlay field lowers to $2E-class bytes ----------------

#[test]
fn field_access_headline_subq() {
    // `timer` is the first overlay field over `sst_custom` (window $2E), so its
    // in-memory offset is $2E. `subq.b #1, timer(a0)` must lower byte-identically
    // to `subq.b #1, $2E(a0)`: SUBQ.B #1,(d16,A0) = 0x5328, ext word $002E, RTS.
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p (a0: *Sst) {{\n    subq.b #1, timer(a0)\n    rts\n}}\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    assert_eq!(proc_bytes(&module), vec![0x53, 0x28, 0x00, 0x2E, 0x4E, 0x75]);
}

// ---- 2. direct struct field ----------------------------------------------

#[test]
fn field_access_direct_struct_field() {
    // `x_pos` is a DIRECT field of `Sst` at offset $10 — resolves in field space
    // with no overlay involved. `move.w x_pos(a0), d0` = 0x3028, ext $0010, RTS.
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p (a0: *Sst) {{\n    move.w x_pos(a0), d0\n    rts\n}}\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    assert_eq!(proc_bytes(&module), vec![0x30, 0x28, 0x00, 0x10, 0x4E, 0x75]);
}

// ---- 3. overlay field at a non-zero overlay offset -----------------------

#[test]
fn field_access_overlay_field_nonzero_offset() {
    // `charge: u16` follows `timer: u8`, so its overlay-relative offset is 1;
    // in-memory offset = window $2E + 1 = $2F. `move.w charge(a0), d0` = 0x3028,
    // ext $002F, RTS. This ALSO legitimately warns `[layout.odd-field]` (the u16
    // lands at an odd memory offset) — a WARNING, not an error.
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8, charge: u16 }}\n\
         proc p (a0: *Sst) {{\n    move.w charge(a0), d0\n    rts\n}}\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    assert_eq!(proc_bytes(&module), vec![0x30, 0x28, 0x00, 0x2F, 0x4E, 0x75]);
    let all = msgs(&src);
    assert!(
        all.iter().any(|m| m.contains("[layout.odd-field]") && m.contains("charge")),
        "want the overlay odd-field warning on `charge`, got: {all:?}"
    );
}

// ---- 4. byte-neutrality (D6.A10) -----------------------------------------

#[test]
fn field_access_is_byte_neutral_with_literal() {
    // `timer(a0)` (field name) and `$2E(a0)` (integer literal) must emit
    // byte-identical images — the field name takes the identical DispInd path.
    let named = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p (a0: *Sst) {{\n    subq.b #1, timer(a0)\n    rts\n}}\n"
    );
    let literal = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p (a0: *Sst) {{\n    subq.b #1, $2E(a0)\n    rts\n}}\n"
    );
    let (m_named, e_named) = lower_errors(&named);
    let (m_lit, e_lit) = lower_errors(&literal);
    assert!(e_named.is_empty(), "named errors: {e_named:?}");
    assert!(e_lit.is_empty(), "literal errors: {e_lit:?}");
    assert_eq!(proc_bytes(&m_named), proc_bytes(&m_lit), "field name must be byte-neutral");
}

// ---- 5. no const fallback on a typed register ----------------------------

#[test]
fn field_access_no_const_fallback_on_typed_reg() {
    // A module-level `const timer` shadows nothing: on a TYPED register the
    // displacement resolves ONLY in field space. With no overlay in scope and no
    // direct field `timer`, this is `[operand.unknown-field]` naming `*Sst` — it
    // must NOT silently use the const's value.
    let src = format!(
        "module m\n{SST}const timer: u8 = 9\n\
         proc p (a0: *Sst) {{\n    tst.b timer(a0)\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| m.contains("[operand.unknown-field]") && m.contains("*Sst")),
        "want [operand.unknown-field] naming *Sst (no const fallback), got: {errs:?}"
    );
}

// ---- 6. untyped register keeps today's semantics -------------------------

#[test]
fn field_access_untyped_reg_comptime_evals_const() {
    // On an UNTYPED register (`a1: *u8` — pointee not a struct), a bare-identifier
    // displacement keeps today's comptime-eval semantics: `MYCONST` ($20) is used
    // as the displacement. `tst.b $20(a1)` = 0x4A29, ext $0020, RTS.
    let src = "module m\n\
        const MYCONST: u8 = $20\n\
        proc p (a1: *u8) {\n    tst.b MYCONST(a1)\n    rts\n}\n";
    let (module, errs) = lower_errors(src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    assert_eq!(proc_bytes(&module), vec![0x4A, 0x29, 0x00, 0x20, 0x4E, 0x75]);
}

#[test]
fn field_access_untyped_reg_does_not_consult_field_space() {
    // Even with an overlay IN SCOPE, a bare field name on an UNTYPED register is
    // NOT resolved in field space — it falls to today's comptime eval, which
    // errors as a plain unknown NAME (not `[operand.unknown-field]`).
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p (a1: *u8) {{\n    tst.b timer(a1)\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| m.contains("timer")),
        "want an unknown-name error on `timer`, got: {errs:?}"
    );
    assert!(
        !errs.iter().any(|m| m.contains("[operand.unknown-field]")),
        "field space must not be consulted for a non-struct pointee, got: {errs:?}"
    );
}

// ---- 7. ambiguous field --------------------------------------------------

#[test]
fn field_access_ambiguous_field() {
    // Two overlays over `sst_custom` both declare `timer`; a bare `timer(a0)`
    // cannot choose between them → `[operand.ambiguous-field]` naming both
    // qualified candidates.
    let src = format!(
        "module m\n{SST}vars V1: sst_custom {{ timer: u8 }}\n\
         vars V2: sst_custom {{ timer: u8 }}\n\
         proc p (a0: *Sst) {{\n    tst.b timer(a0)\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| {
            m.contains("[operand.ambiguous-field]")
                && m.contains("V1.timer")
                && m.contains("V2.timer")
        }),
        "want [operand.ambiguous-field] naming V1.timer and V2.timer, got: {errs:?}"
    );
}

// ---- 8. field-overrun ----------------------------------------------------

#[test]
fn field_access_overrun_wider_than_field() {
    // `move.w timer(a0), d0` reads 2 bytes but `timer` is a 1-byte field — it
    // crosses the named boundary → `[operand.field-overrun]` (D6.A6).
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p (a0: *Sst) {{\n    move.w timer(a0), d0\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| m.contains("[operand.field-overrun]") && m.contains("timer")),
        "want [operand.field-overrun] naming timer, got: {errs:?}"
    );
}

#[test]
fn field_access_narrower_than_field_is_legal() {
    // `move.b charge(a0), d0` reads 1 byte of a 2-byte field — the big-endian
    // high-byte idiom is legal with no lint. `charge` at overlay-rel 1 → mem $2F.
    // move.b (d16,a0),d0 = 0x1028, ext $002F, RTS.
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8, charge: u16 }}\n\
         proc p (a0: *Sst) {{\n    move.b charge(a0), d0\n    rts\n}}\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "narrower access must be legal, got: {errs:?}");
    assert_eq!(proc_bytes(&module), vec![0x10, 0x28, 0x00, 0x2F, 0x4E, 0x75]);
}

// ---- 9. per-proc register typing is scoped (T6 review ride-along) ---------

#[test]
fn field_access_reg_typing_is_per_proc() {
    // `a0` is `*Sst` in proc `p1` but has NO param binding in proc `p2`. The
    // register→struct map is rebuilt per proc, so `p2`'s `timer(a0)` must NOT
    // resolve in field space — it falls to comptime eval and errors as a plain
    // unknown NAME (not `[operand.unknown-field]`). A leaked binding would let
    // `p2` silently resolve `timer` against `*Sst`.
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p1 (a0: *Sst) {{\n    tst.b timer(a0)\n    rts\n}}\n\
         proc p2 () {{\n    tst.b timer(a0)\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| m.contains("timer")),
        "want an unknown-name error on `timer` in p2, got: {errs:?}"
    );
    assert!(
        !errs.iter().any(|m| m.contains("[operand.unknown-field]")),
        "p2's untyped a0 must not consult field space, got: {errs:?}"
    );
}

#[test]
fn field_access_multi_param_types_the_right_register() {
    // One proc with TWO typed params: `a0: *Sst`, `a1: *Other`. `timer` is an
    // Sst-overlay field, NOT any field of `Other` — so `timer(a1)` resolves in
    // `*Other`'s field space, misses, and the `[operand.unknown-field]` error
    // must name `*Other` (a0's binding must not leak onto a1).
    let src = format!(
        "module m\n{SST}\
         struct Other (size: 4) {{ flag: u8, _pad: [u8; 3] @ 1 }}\n\
         vars V: sst_custom {{ timer: u8 }}\n\
         proc p (a0: *Sst, a1: *Other) {{\n    tst.b timer(a1)\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| m.contains("[operand.unknown-field]") && m.contains("*Other")),
        "want [operand.unknown-field] naming *Other (not *Sst), got: {errs:?}"
    );
    assert!(
        !errs.iter().any(|m| m.contains("*Sst")),
        "a0's *Sst binding must not leak onto a1, got: {errs:?}"
    );
}

// =========================================================================
// Task 7 — qualified field access on ANY address register (D6.A4).
//
// A TWO-segment displacement `Overlay.field(aN)` / `Struct.field(aN)` resolves
// in FIELD SPACE explicitly — the qualification IS the author's type assertion,
// so it is legal on ANY address register (typed or not). Resolution: first
// segment names an indexed overlay → resolve the second among ITS fields
// (disp = window_offset + field offset); else names a struct → resolve among
// its DIRECT fields (disp = field offset); else → existing comptime eval,
// byte-for-byte unchanged (preserves e.g. `offsets` ordinals as displacements).
// =========================================================================

// ---- 1. overlay-qualified access on an UNTYPED register ------------------

#[test]
fn qualified_overlay_field_on_untyped_reg() {
    // `a1` has NO param binding, yet `V.timer(a1)` resolves: the qualification
    // is the type assertion. `timer` is the first overlay field over `sst_custom`
    // (window $2E), so disp = $2E. `tst.b V.timer(a1)` = 0x4A29, ext $002E, RTS.
    // (TST.B = 0100 1010 00 mmm rrr; d16(An) mode=101, a1 reg=001 → 0x4A29.)
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p () {{\n    tst.b V.timer(a1)\n    rts\n}}\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    assert_eq!(proc_bytes(&module), vec![0x4A, 0x29, 0x00, 0x2E, 0x4E, 0x75]);
}

// ---- 2. struct-qualified access on an UNTYPED register -------------------

#[test]
fn qualified_struct_field_on_untyped_reg() {
    // `Sst.x_pos(a1)` — struct-qualified direct field at offset $10, untyped reg.
    // `tst.b Sst.x_pos(a1)` = 0x4A29, ext $0010, RTS.
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p () {{\n    tst.b Sst.x_pos(a1)\n    rts\n}}\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    assert_eq!(proc_bytes(&module), vec![0x4A, 0x29, 0x00, 0x10, 0x4E, 0x75]);
}

// ---- 3. qualified form disambiguates the two-overlay case ----------------

#[test]
fn qualified_form_disambiguates_ambiguous_overlays() {
    // Two overlays over `sst_custom` both declare `timer` (bare `timer(a0)` is
    // [operand.ambiguous-field], see Task 6). The qualified `V1.timer(a0)`
    // resolves cleanly — no ambiguity. Here a0 IS typed `*Sst`, proving the
    // qualified form works on a typed register too.
    let src = format!(
        "module m\n{SST}vars V1: sst_custom {{ timer: u8 }}\n\
         vars V2: sst_custom {{ timer: u8 }}\n\
         proc p (a0: *Sst) {{\n    tst.b V1.timer(a0)\n    rts\n}}\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "qualified form must disambiguate, got: {errs:?}");
    assert_eq!(proc_bytes(&module), vec![0x4A, 0x28, 0x00, 0x2E, 0x4E, 0x75]);
}

// ---- 4. regression: non-field two-segment path stays comptime ------------

#[test]
fn qualified_offsets_ordinal_stays_comptime() {
    // `T` is an `offsets` table, NOT an overlay/struct — so `T.B(a0)` must keep
    // today's comptime meaning: `T.B` is ordinal 1, used as the displacement.
    // Field space only claims OVERLAY/STRUCT first segments. `tst.b $1(a0)` =
    // 0x4A28, ext $0001, RTS. (a0 IS typed, proving field space is not consulted
    // for an offsets first-segment.)
    let src = "module m\n\
        data x: [u8;1] = [$AA]\n\
        data y: [u8;1] = [$BB]\n\
        offsets T { A: x, B: y }\n\
        struct S (size: 4) { f: u8, _pad: [u8; 3] @ 1 }\n\
        proc p (a0: *S) {\n    tst.b T.B(a0)\n    rts\n}\n";
    let (module, errs) = lower_errors(src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    // The proc is the LAST item; assert the instruction bytes appear in the image
    // with ext word $0001 (ordinal 1). Locate the tst.b opcode 0x4A28.
    let bytes = proc_bytes(&module);
    let pos = bytes
        .windows(4)
        .position(|w| w == [0x4A, 0x28, 0x00, 0x01])
        .unwrap_or_else(|| panic!("want tst.b $1(a0) = 4A 28 00 01 in {bytes:?}"));
    assert_eq!(&bytes[pos..pos + 4], &[0x4A, 0x28, 0x00, 0x01]);
}

// ---- 5. unknown second segment names the qualifier -----------------------

#[test]
fn qualified_overlay_unknown_field_names_overlay() {
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p () {{\n    tst.b V.nope(a1)\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| m.contains("[operand.unknown-field]") && m.contains("V")),
        "want [operand.unknown-field] naming overlay V, got: {errs:?}"
    );
}

#[test]
fn qualified_struct_unknown_field_names_struct() {
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p () {{\n    tst.b Sst.nope(a1)\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| m.contains("[operand.unknown-field]") && m.contains("Sst")),
        "want [operand.unknown-field] naming struct Sst, got: {errs:?}"
    );
}

// ---- 6. overrun through qualified access ---------------------------------

#[test]
fn qualified_field_overrun() {
    // `move.w V.timer(a1), d0` reads 2 bytes of a 1-byte field → overrun, exactly
    // as in the bare form (the qualified path runs the same overrun check).
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p () {{\n    move.w V.timer(a1), d0\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| m.contains("[operand.field-overrun]") && m.contains("timer")),
        "want [operand.field-overrun] naming timer, got: {errs:?}"
    );
}

// ---- 9. sized overlay-write override (`field:l`) — sst-usability-batch ----

// A struct with a leading u8 and room to overlay: `a:l` writes 4 bytes from
// `a`'s offset, a deliberate multi-field overlay (the load_object $20-$23 idiom).
const OVL: &str = "struct Ovl (size: 8) {\n    \
    a: u8 @ 0,\n    b: u8 @ 1,\n    c: u8 @ 2,\n    d: u8 @ 3,\n    e: u32 @ 4,\n}\n";

#[test]
fn sized_override_authorizes_wide_overlay_write() {
    // `move.l a:l(a0)` — `a` is 1 byte, `:l` declares a 4-byte overlay → NO
    // overrun, displacement is `a`'s offset (0), same as offsetof would give.
    let src = format!(
        "module m\n{OVL}proc p (a0: *Ovl) {{\n    move.l #$FF000000, a:l(a0)\n    rts\n}}\n"
    );
    let (_module, errs) = lower(&src);
    assert!(
        !errs.iter().any(|m| m.contains("[operand.field-overrun]")),
        "the :l override must authorize the wide write: {errs:?}"
    );
}

#[test]
fn sized_override_is_a_stated_width_not_a_mute_switch() {
    // `move.l a:w(a0)` — the override declares 2 bytes but the instruction reads
    // 4 → STILL an overrun (4 > 2). The override is a stated width, not blanket opt-out.
    let src = format!(
        "module m\n{OVL}proc p (a0: *Ovl) {{\n    move.l #$FF000000, a:w(a0)\n    rts\n}}\n"
    );
    let (_module, errs) = lower(&src);
    assert!(
        errs.iter().any(|m| m.contains("[operand.field-overrun]")),
        "move.l against a :w override must still overrun: {errs:?}"
    );
}

#[test]
fn sized_override_narrower_access_is_legal() {
    // `move.b a:l(a0)` — access 1 <= override 4 → clean (documents intent harmlessly).
    let src = format!(
        "module m\n{OVL}proc p (a0: *Ovl) {{\n    move.b #$FF, a:l(a0)\n    rts\n}}\n"
    );
    let (_module, errs) = lower(&src);
    assert!(
        !errs.iter().any(|m| m.contains("[operand.field-overrun]")),
        "narrower access under an override is legal: {errs:?}"
    );
}

#[test]
fn sized_override_bounded_by_struct_end() {
    // RIDER: `d:l(a0)` where `d` is at offset 3 of an 8-byte struct — the :l
    // overlay (offset 3 + 4 = 7 <= 8) is fine; but a field too near the end
    // must fail. Use a 4-byte struct: `d` at 3, :l → 3+4=7 > 4 → past struct end.
    let src = format!(
        "module m\nstruct Tiny (size: 4) {{\n    a: u8 @ 0,\n    b: u8 @ 1,\n    c: u8 @ 2,\n    d: u8 @ 3,\n}}\n\
         proc p (a0: *Tiny) {{\n    move.l #$FF000000, d:l(a0)\n    rts\n}}\n"
    );
    let (_module, errs) = lower(&src);
    assert!(
        errs.iter().any(|m| m.contains("[operand.field-overrun]") && m.contains("d")),
        "an overlay running past the struct end must be caught: {errs:?}"
    );
}
