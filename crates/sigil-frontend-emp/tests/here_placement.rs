//! The label-equivalence invariant for `here()` under PLACEMENT (L-H.1): a
//! `here()` resolves to exactly what a label written at the same position
//! resolves to, whatever moves the section after lowering.
//!
//! A label's address is `Section::vma_origin() + offset`, and `vma_origin()` is
//! `vma_base.unwrap_or(lma)`. For a section without an explicit `vma:`, that is
//! `lma`, and three passes rewrite `lma` after lowering: `place_sequential` (the
//! CLI's no-map path, base 0), `place_sections` (the `--map` path), and the
//! linker's `place_pass` (a Chained section lands wherever its predecessor's
//! FINAL size ends). So for such a section the base is a link-time fact, and
//! `here()` must be a link-time value there. Only a section whose `vma:` is
//! explicit has a base known at lowering time; there `here()` stays an exact
//! comptime integer.
//!
//! Each differential test reports what BOTH arms produced, and asserts that the
//! arrangement actually moved the section (so agreement cannot be the vacuous
//! "both read module-local zero").

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::{build_program, manifest::Manifest, place_sequential};
use sigil_ir::backend::Cpu;
use sigil_ir::{LinkAssert, Module, Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn opts() -> LowerOptions {
    LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] }
}

fn errors(diags: &[Diagnostic]) -> Vec<String> {
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

/// Lower a single-file module; every diagnostic is returned as text.
fn lower_one(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) = lower_module(&file, &opts());
    (m, diags.into_iter().map(|d| d.message).collect())
}

/// Build a multi-file program the way the CLI's no-map path does: `build_program`
/// then `place_sequential(.., 0)` (main.rs, the `emp` subcommand without `--map`).
fn build_placed(files: &[(&str, &str)], entry: &str) -> (Vec<Section>, Vec<LinkAssert>, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    for (rel, content) in files {
        let p = dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }
    let (manifest, mdiags) = Manifest::scan(dir.path());
    assert!(errors(&mdiags).is_empty(), "manifest errors: {:?}", errors(&mdiags));
    let (mut sections, asserts, diags) = build_program(&manifest, entry, None, &opts());
    place_sequential(&mut sections, 0);
    (sections, asserts, errors(&diags))
}

/// The two arms at one position. `label`: the address the linker gives the label
/// (`vma_origin() + offset` of the RESOLVED section, which is what every fixup
/// against that label reads). `here`: the big-endian u32 the data item emitted,
/// which is what `here()` produced. Also returns the resolved section's `lma`, so
/// a caller can prove the arrangement moved the section.
struct Arms {
    label: u32,
    here: u32,
    section_lma: u32,
}

fn arms(sections: &[Section], label: &str) -> Arms {
    let resolved = sigil_link::resolve_layout(sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    // A whole-program build canonically renames a module's labels to
    // `<module>.<name>`; a single-file lower keeps the plain name.
    let qualified = format!(".{label}");
    let (sec, lab) = resolved
        .iter()
        .find_map(|s| {
            s.labels.iter().find(|l| l.name == label || l.name.ends_with(&qualified)).map(|l| (s, l))
        })
        .unwrap_or_else(|| panic!("label {label} not found in any resolved section"));
    let bytes = &linked.section(&sec.name).expect("linked section").bytes;
    let at = lab.offset as usize;
    let here = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
    Arms { label: sec.vma_origin() + lab.offset, here, section_lma: sec.lma }
}

const HEAD: &str = "\
module probe.a
use probe.b.{Tag}
data Head: [u8; 16] = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]
proc init () {
  lea Tag, a0
  rts
}
";

/// A section without `vma:` in a module the whole-program placer lands AFTER
/// another module's bytes. The label follows placement; `here()` must too.
#[test]
fn here_in_a_placed_module_section_equals_its_own_label() {
    let b = "\
module probe.b
section blob (cpu: m68000) {
  pub data Tag: u32 = here()
}
";
    let (sections, _asserts, errs) = build_placed(&[("probe/a.emp", HEAD), ("probe/b.emp", b)], "probe.a");
    assert!(errs.is_empty(), "build errors: {errs:?}");
    let a = arms(&sections, "Tag");
    // Non-vacuity: module a emits 16 data bytes plus a proc ahead of `blob`, so
    // the placer must have moved `blob` at least 16 bytes past module-local 0.
    // If this fails the arrangement is broken and the differential below is
    // meaningless, which is why it is checked first.
    assert!(
        a.section_lma >= 16,
        "arrangement: `blob` should be placed past module a's 16-byte Head, its lma is {:#x}",
        a.section_lma
    );
    assert_eq!(
        a.here, a.label,
        "here() and the label Tag at the same position disagree: label resolves to {:#x}, here() emitted {:#x}",
        a.label, a.here
    );
}

/// The capacity-guard shape the finding names: `ensure(here() <= LIMIT)` in a
/// section without `vma:`. A `here()` read at module-local 4 passes a limit of 8
/// forever; the placed position (16-byte Head + a proc + 4) is past it, so the
/// guard must FAIL, and fail at link where the placed address is known.
#[test]
fn capacity_guard_on_here_in_a_placed_section_fails_at_the_placed_address() {
    let b = "\
module probe.b
section blob (cpu: m68000) {
  pub data Tag: u32 = 0
  ensure(here() <= 8, \"blob overran its 8-byte budget at {here()}\")
}
";
    let (sections, asserts, errs) = build_placed(&[("probe/a.emp", HEAD), ("probe/b.emp", b)], "probe.a");
    assert!(errs.is_empty(), "the guard must not be decided at lowering, but lowering errored: {errs:?}");
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true).expect("resolve_layout");
    let a = arms(&sections, "Tag");
    assert!(a.label + 4 > 8, "arrangement: the placed guard position {:#x} must exceed the budget", a.label + 4);
    let verdicts = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &asserts);
    let failed: Vec<&String> = verdicts
        .iter()
        .filter(|d| d.level == Level::Error && d.message.contains("blob overran"))
        .map(|d| &d.message)
        .collect();
    assert!(
        !failed.is_empty(),
        "the guard passed silently: {} deferred assert(s), verdicts {:?}",
        asserts.len(),
        verdicts.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// Single module, two sections, no whole-program placer: section `a` holds a
/// `jbra` that grows at link (2 to 4 bytes), and section `b` is Chained after it
/// with no relaxable of its own. `b`'s label moves with `a`'s growth; `here()`
/// in `b` must move with it, even though `b` itself holds nothing relaxable.
#[test]
fn here_in_a_chained_section_follows_the_previous_section_growth() {
    let src = "\
module m
section a (cpu: m68000) {
  proc p () {
    jbra Far
  }
  data Pad = bytes(for i in 0..200 { 0 })
  proc Far () {
    rts
  }
}
section b (cpu: m68000) {
  data Tag: u32 = here()
}
";
    let (m, diags) = lower_one(src);
    assert!(diags.is_empty(), "lower: {diags:?}");
    let a = arms(&m.sections, "Tag");
    // Derived from the source: bra.w (4) + 200 pad bytes + rts (2) = 206. The
    // lowering baseline counts the jbra at its smallest rung (2), so a stale
    // here() would read 204.
    assert_eq!(a.label, 4 + 200 + 2, "arrangement: section b's label should sit after a's GROWN size");
    assert_eq!(
        a.here, a.label,
        "here() and the label Tag at the same position disagree: label resolves to {:#x}, here() emitted {:#x}",
        a.label, a.here
    );
}

/// The exact fast path survives exactly where the base is known at lowering: a
/// section with an explicit `vma:`. There `here()` is a comptime integer and may
/// size an array (a use a link-time value cannot serve).
#[test]
fn here_in_a_vma_pinned_section_is_an_exact_comptime_integer() {
    let src = "\
module m
section s (cpu: m68000, vma: $8000) {
  data A: [u8; 4] = [1,2,3,4]
  data B: [u8; here() - $8000] = [5,6,7,8]
}
";
    let (m, diags) = lower_one(src);
    assert!(diags.is_empty(), "a pinned-section here() must stay exact, got: {diags:?}");
    let resolved = sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    assert_eq!(linked.section("s").unwrap().bytes, vec![1, 2, 3, 4, 5, 6, 7, 8]);
}

/// In a section without `vma:` the base is a link-time fact, so a `here()` that
/// must be a comptime integer (an array length) is the `[here.provisional]`
/// refusal (D-H.2), never a module-local number baked in silently.
#[test]
fn here_sizing_an_array_in_an_unpinned_section_refuses() {
    let src = "\
module m
section s (cpu: m68000) {
  data A: [u8; 4] = [1,2,3,4]
  data B: [u8; here()] = []
}
";
    let (_m, diags) = lower_one(src);
    assert!(
        diags.iter().any(|d| d.contains("[here.provisional]")),
        "expected [here.provisional] for a comptime-sizing here() in an unpinned section, got: {diags:?}"
    );
}
