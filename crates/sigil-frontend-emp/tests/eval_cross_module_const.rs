//! L5 (Spec 2 language round): a `pub const`'s initializer resolves in its
//! DEFINING module's scope, so a derived value (`= OTHER_CONST + n`) referencing
//! a same-module sibling survives a `use`-import into a consumer that imports
//! only the derived const, not the sibling.
//!
//! Before L5 the injected clone re-evaluated its expression in the consumer's
//! scope, where the sibling was invisible — `unknown name`. The fix folds the
//! value at the definition site (the const-value analogue of the overlay-window
//! stamp), so the consumer reads a self-contained literal and no sibling name
//! leaks into its namespace.

use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{build_program, manifest::Manifest};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

fn build(files: &[(&str, &str)], entry: &str) -> (Vec<Section>, Vec<Diagnostic>) {
    let dir = tempfile::tempdir().unwrap();
    for (rel, content) in files {
        write(dir.path(), rel, content);
    }
    let (manifest, mdiags) = Manifest::scan(dir.path());
    assert!(
        mdiags.iter().all(|d| d.level != Level::Error),
        "manifest errors: {:?}",
        mdiags.iter().filter(|d| d.level == Level::Error).collect::<Vec<_>>()
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (sections, _asserts, diags) = build_program(&manifest, entry, None, &opts);
    (sections, diags)
}

fn errors(diags: &[Diagnostic]) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.level == Level::Error)
        .map(|d| d.message.clone())
        .collect()
}

/// Flatten the whole program to a single byte image.
fn flatten(sections: &[Section]) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// The consumer's `data D: u16 = <imported const>` lowers to the value's two
/// big-endian bytes; return them as a u16 for a direct value assertion.
fn consumer_value(files: &[(&str, &str)]) -> (u16, Vec<String>) {
    let (sections, diags) = build(files, "b");
    let errs = errors(&diags);
    if !errs.is_empty() {
        return (0, errs);
    }
    let img = flatten(&sections);
    assert_eq!(img.len(), 2, "expected one u16 data item, got {} bytes", img.len());
    (u16::from_be_bytes([img[0], img[1]]), errs)
}

// ---- positive: a derived initializer resolves its sibling at home ----------

#[test]
fn typed_const_refs_typed_sibling() {
    // The demand site (games.sonic4.constants VRAM_TEST_MARKER = VRAM_TEST_OBJ + $18):
    // a typed const derived from another typed const of the same module.
    let a = "\
module a
newtype VramTile = u16
pub const BASE: VramTile = $03E0
pub const MARKER: VramTile = BASE + $18
";
    let b = "\
module b
use a.{MARKER}
data D: u16 = MARKER
";
    let (v, errs) = consumer_value(&[("a.emp", a), ("b.emp", b)]);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    assert_eq!(v, 0x03F8);
}

#[test]
fn typed_const_refs_untyped_sibling() {
    let a = "\
module a
newtype VramTile = u16
pub const RAW = $10
pub const MARKER: VramTile = RAW + $08
";
    let b = "\
module b
use a.{MARKER}
data D: u16 = MARKER
";
    let (v, errs) = consumer_value(&[("a.emp", a), ("b.emp", b)]);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    assert_eq!(v, 0x0018);
}

#[test]
fn chain_of_two_siblings_resolves() {
    // MARKER depends on MID depends on BASE — a two-hop derivation; the consumer
    // imports only MARKER.
    let a = "\
module a
pub const BASE = 1
pub const MID = BASE + 1
pub const MARKER = MID + 1
";
    let b = "\
module b
use a.{MARKER}
data D: u16 = MARKER
";
    let (v, errs) = consumer_value(&[("a.emp", a), ("b.emp", b)]);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    assert_eq!(v, 3);
}

// ---- leak-free: a NON-imported sibling stays invisible in the consumer -----

#[test]
fn unimported_sibling_does_not_leak_into_consumer() {
    // B imports MARKER (which the fix folds to a literal), but references BASE
    // directly WITHOUT importing it. The fold must NOT have injected BASE into
    // B's scope — a bare `BASE` is still `unknown name`.
    let a = "\
module a
pub const BASE = $03E0
pub const MARKER = BASE + $18
";
    let b = "\
module b
use a.{MARKER}
data D: u16 = BASE
";
    let (_v, errs) = consumer_value(&[("a.emp", a), ("b.emp", b)]);
    assert!(
        errs.iter().any(|e| e.contains("unknown name") && e.contains("BASE")),
        "expected `unknown name BASE` (no sibling leak), got {errs:?}"
    );
}

// ---- negative: unresolvable / cyclic initializers stay loud ----------------

#[test]
fn unknown_name_in_initializer_still_errors_cleanly() {
    // MARKER's initializer names something that exists nowhere. The home fold
    // cannot resolve it (falls back to the raw expression), so the consumer's
    // own eval reports the miss — loud, not silently swallowed.
    let a = "\
module a
pub const MARKER = NOPE + 1
";
    let b = "\
module b
use a.{MARKER}
data D: u16 = MARKER
";
    let (_v, errs) = consumer_value(&[("a.emp", a), ("b.emp", b)]);
    assert!(
        errs.iter().any(|e| e.contains("unknown name") && e.contains("NOPE")),
        "expected `unknown name NOPE`, got {errs:?}"
    );
}

#[test]
fn cyclic_initializer_is_a_loud_diagnostic() {
    // P and Q reference each other. The fix must not hang: the fold detects the
    // cycle (and discards its diagnostics, falling back to the raw expression),
    // and the consumer's resolution reports the cyclic const definition. Both are
    // imported so the cycle is reachable in the consumer's scope (a lone import of
    // one arm errors `unknown name` on the other — still loud, never a hang).
    let a = "\
module a
pub const P = Q + 1
pub const Q = P + 1
";
    let b = "\
module b
use a.{P, Q}
data D: u16 = P
";
    let (_v, errs) = consumer_value(&[("a.emp", a), ("b.emp", b)]);
    assert!(
        errs.iter().any(|e| e.contains("cyclic const definition")),
        "expected a cyclic-const diagnostic, got {errs:?}"
    );
}
