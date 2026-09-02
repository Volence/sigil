//! An `Enum.Variant` argument in a `pub const` initializer, where the enum was
//! imported from another module.
//!
//! A `pub const` travels to its importers as a CLONE, and before cloning the
//! resolver runs a definition-site FOLD PROBE — it evaluates the const against
//! its defining FILE alone (`resolve::fold_const_literal`). That scope is
//! deliberately narrower than the real one: the defining module's own `use`
//! imports are absent. A name the probe cannot see is expected to miss
//! HARMLESSLY (`resolve::is_probe_scope_shortfall` filters the miss out), and
//! the consumer resolves the same expression in its own, wider scope.
//!
//! The D-PP.3 label fallback destroys the evidence of one such miss. In a call
//! argument an unresolvable path is not an `unknown name` — it becomes a
//! `Value::Label`, and the parameter-class check then refuses it with a message
//! about LABELS, which names no missing symbol. So a shortfall was reported as a
//! fault in the const's definition and the build went red, even though every
//! real consumer could resolve the path perfectly.
//!
//! Each row asserts the EMITTED WORD, not the absence of a diagnostic: the two
//! variants map to different bytes, so a fix that accepts the const while
//! binding the wrong value fails here too.

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
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

fn flatten(sections: &[Section]) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// Build the program with `c` as the entry and read its single `data D: u16`.
/// Returns the emitted word plus any errors (the word is meaningless when the
/// error list is non-empty, so every caller asserts on the errors first).
fn entry_word(files: &[(&str, &str)]) -> (u16, Vec<String>) {
    let (sections, diags) = build(files, "c");
    let errs = errors(&diags);
    if !errs.is_empty() {
        return (0, errs);
    }
    let img = flatten(&sections);
    assert_eq!(img.len(), 2, "expected one u16 data item, got {} bytes", img.len());
    (u16::from_be_bytes([img[0], img[1]]), errs)
}

/// Module A — the enum's home (aeon's `engine.level.scene_dsl`): the enum, a
/// `pub` fn of its own that takes it, and a `Label`-typed fn for the over-fire
/// rows. The two variants answer with distinct, non-zero words, so a mis-bound
/// argument shows up in the byte and not only in the diagnostics.
const HOME: &str = "\
module a
pub comptime enum Mask { Undeclared, Accept }
pub comptime fn mask_word(m: Mask) -> int {
    return match m { Undeclared => $1111, Accept => $2222 }
}
";

/// Build with `b` as the entry — the shapes that need NO importing consumer,
/// so the fold probe never runs and the call is bound in the only scope there
/// is. Reads `b`'s single `data D: u16`.
fn own_module_word(b: &str) -> (u16, Vec<String>) {
    let (sections, diags) = build(&[("a.emp", HOME), ("b.emp", b)], "b");
    let errs = errors(&diags);
    if !errs.is_empty() {
        return (0, errs);
    }
    let img = flatten(&sections);
    assert_eq!(img.len(), 2, "expected one u16 data item, got {} bytes", img.len());
    (u16::from_be_bytes([img[0], img[1]]), errs)
}

/// Module C — the importing consumer (aeon's `games.sonic4.scene_registry`),
/// which globs BOTH the enum's home and the module holding the const. Its scope
/// resolves everything; only the definition-site fold probe's narrower one does
/// not.
const CONSUMER: &str = "\
module c
use a.*
use b.*
data D: u16 = K
";

/// Row 1 — the reported shape. Module B globs the enum's home, declares its own
/// wrapper taking the imported enum, and spells the variant as a NAMED argument
/// in a `pub const` another module imports.
#[test]
fn glob_imported_enum_variant_in_a_pub_const_initializer() {
    let b = "\
module b
use a.*
pub comptime fn wrap(m: Mask) -> int { return mask_word(m) }
pub const K: u16 = wrap(m: Mask.Accept)
";
    let (v, errs) = entry_word(&[("a.emp", HOME), ("b.emp", b), ("c.emp", CONSUMER)]);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    assert_eq!(v, 0x2222, "the bound argument must be `Accept`, not `Undeclared`");
}

/// Row 2 — the same const with a NAMED import instead of the glob, reported as
/// no help either.
#[test]
fn named_imported_enum_variant_in_a_pub_const_initializer() {
    let b = "\
module b
use a.{Mask, mask_word}
pub comptime fn wrap(m: Mask) -> int { return mask_word(m) }
pub const K: u16 = wrap(m: Mask.Accept)
";
    let (v, errs) = entry_word(&[("a.emp", HOME), ("b.emp", b), ("c.emp", CONSUMER)]);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    assert_eq!(v, 0x2222, "the bound argument must be `Accept`, not `Undeclared`");
}

/// Row 3 — the other variant through the same path: the value must travel, not
/// just the acceptance.
#[test]
fn the_other_variant_binds_too() {
    let b = "\
module b
use a.*
pub comptime fn wrap(m: Mask) -> int { return mask_word(m) }
pub const K: u16 = wrap(m: Mask.Undeclared)
";
    let (v, errs) = entry_word(&[("a.emp", HOME), ("b.emp", b), ("c.emp", CONSUMER)]);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    assert_eq!(v, 0x1111, "the bound argument must be `Undeclared`, not `Accept`");
}

/// Row 4 — the case that must KEEP working: the same imported path as an
/// argument to a fn declared in the enum's OWN module.
#[test]
fn variant_argument_to_a_fn_in_the_enums_own_module_still_works() {
    let b = "\
module b
use a.*
pub const K: u16 = mask_word(m: Mask.Accept)
";
    let (v, errs) = entry_word(&[("a.emp", HOME), ("b.emp", b), ("c.emp", CONSUMER)]);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    assert_eq!(v, 0x2222, "the bound argument must be `Accept`, not `Undeclared`");
}

/// Row 5 — the case that must keep working WITHOUT the fold probe in the
/// picture: the whole call inside one module, so the value is bound in the only
/// scope there is.
#[test]
fn same_module_variant_argument_still_works() {
    let b = "\
module b
use a.*
comptime fn wrap(m: Mask) -> int { return mask_word(m) }
data D: u16 = wrap(m: Mask.Accept)
";
    let (v, errs) = own_module_word(b);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    assert_eq!(v, 0x2222, "the same-module call must still bind `Accept`");
}

// ---- over-fire guards ---------------------------------------------------

/// Row 6 — a GENUINE label passed where the parameter is an imported enum must
/// STILL be refused, even though the label reaches the fold probe in exactly the
/// shape an unresolvable name does. The consumer's own evaluation is what has to
/// stay loud: `Handler` is unknown there too, so it is a link symbol, and a link
/// symbol is not a `Mask`.
#[test]
fn a_real_label_at_an_enum_parameter_is_still_refused() {
    let b = "\
module b
use a.*
pub comptime fn wrap(m: Mask) -> int { return mask_word(m) }
pub const K: u16 = wrap(m: Handler)
";
    let (_, errs) = entry_word(&[("a.emp", HOME), ("b.emp", b), ("c.emp", CONSUMER)]);
    assert!(
        errs.iter().any(|e| e.contains("a label is not a valid `Mask` argument")),
        "a label at an enum parameter must still be refused; errors were {errs:?}"
    );
}

/// Row 7 — the same over-fire guard with no fold probe involved at all: a
/// dotted link symbol as an argument, in a data item of the module that owns
/// the call.
#[test]
fn a_dotted_link_symbol_at_an_enum_parameter_is_still_refused() {
    let b = "\
module b
use a.*
comptime fn wrap(m: Mask) -> int { return mask_word(m) }
data D: u16 = wrap(m: elsewhere.entry)
";
    let (_, errs) = own_module_word(b);
    assert!(
        errs.iter().any(|e| e.contains("a label is not a valid `Mask` argument")),
        "a label at an enum parameter must still be refused; errors were {errs:?}"
    );
}

/// Row 8 — a typo'd variant of a genuinely visible enum must still name itself.
#[test]
fn an_unknown_variant_of_an_imported_enum_is_loud() {
    let b = "\
module b
use a.*
comptime fn wrap(m: Mask) -> int { return mask_word(m) }
data D: u16 = wrap(m: Mask.Nope)
";
    let (_, errs) = own_module_word(b);
    assert!(
        errs.iter().any(|e| e.contains("enum `Mask` has no variant `Nope`")),
        "errors were {errs:?}"
    );
}

/// Row 9 — the fold probe's `Failed` net must keep catching a REAL fault in a
/// const's own definition, in the very same const that also mints a fallback
/// label. Silencing the probe wholesale whenever it minted one — the coarse
/// alternative to this parcel's fix — would lose exactly this.
///
/// `NEVER_READ` is the load-bearing detail. The glob import makes the resolver
/// FOLD it (so the probe runs), and nothing demands its value, so no consumer
/// ever evaluates it a second time: the probe is the only thing that can report
/// its fault, and the row goes quiet the moment the probe is silenced.
#[test]
fn a_real_definition_fault_is_still_reported_beside_a_fallback_label() {
    let b = "\
module b
use a.*
pub comptime fn label_word(l: Label) -> int { return $0007 }
pub const K: u16 = 0
pub const NEVER_READ = label_word(Handler) + (1 / 0)
";
    let (_, errs) = entry_word(&[("a.emp", HOME), ("b.emp", b), ("c.emp", CONSUMER)]);
    assert!(
        errs.iter().any(|e| e.contains("division by zero")),
        "the const's own fault must still be reported; errors were {errs:?}"
    );
}
