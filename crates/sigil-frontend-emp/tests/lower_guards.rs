//! Item-position guards: evaluated in item order at lowering time, zero bytes,
//! `ensure_fatal` stops the module's remaining items (D5.2/D5.3).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

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

/// A span-free fingerprint of one section: name, cpu, vma_base, lma, labels
/// (name+offset), and LINKED bytes (fixups resolved).
type SectionFingerprint = (String, String, Option<u32>, u32, Vec<(String, u32)>, Vec<u8>);

/// A span-free fingerprint of a lowered module for byte-neutrality comparison.
/// Spans are excluded because two different source texts legitimately differ in
/// byte offsets while producing byte-identical output.
fn fingerprint(m: &Module) -> Vec<SectionFingerprint> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    m.sections
        .iter()
        .map(|s| {
            let labels = s.labels.iter().map(|l| (l.name.clone(), l.offset)).collect();
            let bytes = linked.section(&s.name).map(|ls| ls.bytes.clone()).unwrap_or_default();
            (s.name.clone(), format!("{:?}", s.cpu), s.vma_base, s.lma, labels, bytes)
        })
        .collect()
}

#[test]
fn failing_top_level_ensure_diagnoses_with_interpolation() {
    let d = msgs("module m\nconst N = 3\nensure(N == 4, \"want 4, got {N}\")\ndata T: [u8;1] = [1]\n");
    let hits: Vec<_> = d.iter().filter(|m| m.contains("want 4, got 3")).collect();
    assert_eq!(hits.len(), 1, "exactly one interpolated diagnostic, got: {d:?}");
}

#[test]
fn passing_guards_are_byte_neutral() {
    // The SAME data, once with guards interleaved (top level AND inside a
    // `section (vma: $8000)` block), once without. The lowered modules — sections,
    // bytes, labels, fixups — must be identical.
    let with_guards = "module m\n\
        const N = 4\n\
        ensure(N == 4, \"ok {N}\")\n\
        data A: [u8;2] = [1,2]\n\
        section blk (vma: $8000) {\n\
        ensure(2 > 1, \"still ok\")\n\
        data B: [u8;2] = [3,4]\n\
        }\n";
    let without_guards = "module m\n\
        const N = 4\n\
        data A: [u8;2] = [1,2]\n\
        section blk (vma: $8000) {\n\
        data B: [u8;2] = [3,4]\n\
        }\n";
    let (m1, d1) = lower(with_guards);
    let (m2, d2) = lower(without_guards);
    assert!(d1.is_empty(), "guarded lower had diags: {d1:?}");
    assert!(d2.is_empty(), "plain lower had diags: {d2:?}");
    assert_eq!(fingerprint(&m1), fingerprint(&m2), "guards must be byte/label/fixup-neutral");
}

#[test]
fn here_in_guard_sees_current_position() {
    // `here()` in a guard reads the item's start VMA. A `section (vma: $8000)` with
    // a 4-byte data item then `ensure(here() == $8004, ...)` — the guard sees the
    // post-data position.
    let d = msgs("module m\nsection s (vma: $8000) {\n\
        data A: [u8; 4] = [1,2,3,4]\n\
        ensure(here() == $8004, \"pos {here()}\")\n\
        }\n");
    assert!(d.is_empty(), "here()-aware guard should pass silently, got: {d:?}");
}

#[test]
fn ensure_fatal_stops_remaining_items() {
    let d = msgs("module m\nensure_fatal(false, \"boom\")\nensure(false, \"later\")\n");
    assert!(d.iter().any(|m| m.contains("boom")), "fatal message present: {d:?}");
    assert!(!d.iter().any(|m| m.contains("later")), "later guard must NOT run: {d:?}");
}

#[test]
fn plain_ensure_failure_continues() {
    let d = msgs("module m\nensure(false, \"first\")\nensure(false, \"second\")\n");
    assert!(d.iter().any(|m| m.contains("first")), "first present: {d:?}");
    assert!(d.iter().any(|m| m.contains("second")), "second present (non-fatal continues): {d:?}");
}

#[test]
fn ensure_fatal_in_section_stops_top_level_items() {
    // A fatal INSIDE a section block must also stop the module's remaining
    // TOP-LEVEL items (D5.3).
    let d = msgs("module m\nsection s (vma: $8000) {\n\
        ensure_fatal(false, \"boom\")\n\
        }\n\
        ensure(false, \"after\")\n");
    assert!(d.iter().any(|m| m.contains("boom")), "fatal present: {d:?}");
    assert!(!d.iter().any(|m| m.contains("after")), "top-level guard after fatal section must NOT run: {d:?}");
}

#[test]
fn guard_sees_offsets_ordinals() {
    let d = msgs("module m\n\
        offsets Idx { A: T1, B: T2 }\n\
        data T1: [u8;1] = [1]\n\
        data T2: [u8;1] = [2]\n\
        ensure(Idx.count == 2, \"count {Idx.count}\")\n");
    assert!(d.is_empty(), "offsets .count guard should pass silently, got: {d:?}");
}

#[test]
fn unknown_name_in_guard_condition_diagnoses_without_crash() {
    // No panic; an unknown-name diagnostic exists; a following non-fatal guard
    // still runs (lowering continues).
    let d = msgs("module m\nensure(nonexistent > 0, \"x\")\nensure(false, \"reached\")\n");
    assert!(!d.is_empty(), "unknown name should diagnose: {d:?}");
    assert!(d.iter().any(|m| m.contains("reached")), "lowering continues after non-fatal: {d:?}");
}
