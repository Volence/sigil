//! L8 (Spec 2 language round): `extern NAME: Type` — a typed reference to a value
//! symbol defined outside the module and resolved at link (a harvested game
//! constant's EquSym, an AS/emp equ). It lets a game-agnostic engine module name a
//! game-side id WITH its newtype, so the value crosses the seam once (no local
//! mirror, no drift guard). At use sites the reference erases to the same link
//! immediate a bare symbol would, carrying the type for enforcement in typed
//! positions.
//!
//! Byte-level coverage of the link path (the `moveq #EXTERN` immediate encoding
//! identically to the bare symbol it replaces) lives in the real build: the
//! six-target ROM byte-identity plus `sound_api_port`'s region gate, where the
//! harvest provisions the game symbol. These unit tests cover what does not need
//! that provisioning — the grammar and the newtype enforcement.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Level;

fn lower_errors(src: &str) -> Vec<String> {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse diags: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (_module, diags) = lower_module(&file, &opts);
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

// ---- parse -----------------------------------------------------------------

#[test]
fn extern_const_parses_with_a_type() {
    let (file, diags) = parse_str("module m\nextern SFXID_RING: SfxId\n");
    assert!(diags.iter().all(|d| d.level != Level::Error), "unexpected parse diags: {diags:?}");
    let has = file.items.iter().any(|it| {
        matches!(it, sigil_frontend_emp::ast::Item::ExternConst(e) if e.name == "SFXID_RING")
    });
    assert!(has, "expected an ExternConst item");
}

#[test]
fn extern_const_requires_a_type() {
    // The whole point is to carry a type — a bare `extern NAME` is a parse error.
    let (_file, diags) = parse_str("module m\nextern SFXID_RING\n");
    assert!(
        diags.iter().any(|d| d.level == Level::Error),
        "expected a parse error for a type-less extern"
    );
}

#[test]
fn extern_disambiguates_from_the_extern_call_and_extern_proc() {
    // `extern("Sym")` in expression position and `extern proc` stay their own forms;
    // only `extern IDENT : Type` is the typed-extern item.
    let src = "\
module m
extern SFXID_RING: SfxId
newtype SfxId = u8
equ ECHO = extern(\"SomeSym\")
extern proc Callee ()
";
    let (file, diags) = parse_str(src);
    assert!(diags.iter().all(|d| d.level != Level::Error), "unexpected parse diags: {diags:?}");
    let externs = file
        .items
        .iter()
        .filter(|it| matches!(it, sigil_frontend_emp::ast::Item::ExternConst(_)))
        .count();
    let extern_procs = file
        .items
        .iter()
        .filter(|it| matches!(it, sigil_frontend_emp::ast::Item::ExternProc(_)))
        .count();
    assert_eq!(externs, 1, "exactly one typed-extern item");
    assert_eq!(extern_procs, 1, "the extern proc stays its own form");
}

// ---- type enforcement ------------------------------------------------------

#[test]
fn typed_extern_into_a_wrong_newtype_data_slot_is_an_error() {
    // The extern carries SfxId; feeding it to a SongId-typed data slot is a type
    // violation — the enforcement a bare link name cannot give. The type check
    // fires at lowering, before any link resolution.
    let errs = lower_errors(
        "\
module engine
extern SFXID_RING: SfxId
newtype SfxId = u8
newtype SongId = u8
data D: SongId = SFXID_RING
",
    );
    assert!(
        errs.iter().any(|e| e.contains("[emit.type]") && e.contains("SongId")),
        "expected an SfxId-into-SongId type error, got {errs:?}"
    );
}

#[test]
fn a_plain_int_still_fills_a_matching_newtype_slot() {
    // Control: the type machinery is not rejecting everything — a plain int in a
    // newtype's own slot lowers cleanly (no `[emit.type]`).
    let errs = lower_errors(
        "\
module engine
newtype SongId = u8
data D: SongId = 1
",
    );
    assert!(errs.is_empty(), "a matching plain-int slot must lower cleanly, got {errs:?}");
}
