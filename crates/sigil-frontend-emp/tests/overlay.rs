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
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
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
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
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
