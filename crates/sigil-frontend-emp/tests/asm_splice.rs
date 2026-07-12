//! `asm{}` Code-splice — `{expr}` at statement position (2026-07-11 mini-spec).
//!
//! A `{expr}` at STATEMENT position inside an `asm { }` block evaluates `expr`
//! in the enclosing comptime scope; it must yield `Code`, whose items are
//! inlined in place. `Code.empty()` splices to nothing. The skeleton's own
//! labels resolve within the skeleton block (hygiene unchanged) — the feature
//! is that a loop skeleton (label + branch) can live in ONE block with the
//! conditional pieces as label-free holes.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn lower(src: &str) -> (sigil_ir::Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(
        perrs.iter().all(|d| d.level != Level::Error),
        "unexpected parse diagnostics: {perrs:?}"
    );
    lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    )
}

fn section<'a>(module: &'a sigil_ir::Module, name: &str) -> &'a Section {
    module
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section `{name}`"))
}

fn label_offset(sec: &Section, name: &str) -> u32 {
    sec.labels
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("no label `{name}`"))
        .offset
}

fn linked_section_bytes(module: &sigil_ir::Module, name: &str) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

fn proc_bytes(module: &sigil_ir::Module, sec: &str, name: &str, len: usize) -> Vec<u8> {
    let s = section(module, sec);
    let off = label_offset(s, name) as usize;
    let linked = linked_section_bytes(module, sec);
    linked[off..off + len].to_vec()
}

fn errors(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.as_str()).collect()
}

/// A `{ helper() }` splice inlines the helper's Code items in place: the
/// helper's `nop` lands between the surrounding instructions.
#[test]
fn splice_inlines_helper_code() {
    let src = "\
module m
comptime fn ins() -> Code { return asm { nop } }
comptime fn wrap() -> Code {
    return asm {
        move.w  #1, d0
        { ins() }
        rts
    }
}
proc p() {
    wrap()
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // move.w #1,d0 = 303C 0001 ; nop = 4E71 ; rts = 4E75
    assert_eq!(
        proc_bytes(&module, "text", "p", 8),
        vec![0x30, 0x3C, 0x00, 0x01, 0x4E, 0x71, 0x4E, 0x75],
        "the `{{ ins() }}` splice must inline the helper's nop between move and rts"
    );
}

/// An empty-Code splice (`{ nothing() }` where the helper returns an empty
/// `asm { }`) inlines zero items — the surrounding instructions stay adjacent.
/// This is the aabb idiom: a helper returns either code or nothing, one splice.
#[test]
fn empty_splice_emits_nothing() {
    let src = "\
module m
comptime fn nothing() -> Code { return asm { } }
comptime fn wrap() -> Code {
    return asm {
        move.w  #1, d0
        { nothing() }
        move.w  #2, d0
        rts
    }
}
proc p() {
    wrap()
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // move.w #1,d0 ; move.w #2,d0 ; rts — no gap where the empty splice sits.
    assert_eq!(
        proc_bytes(&module, "text", "p", 10),
        vec![0x30, 0x3C, 0x00, 0x01, 0x30, 0x3C, 0x00, 0x02, 0x4E, 0x75],
        "an empty-Code splice must inline nothing"
    );
}

/// THE ACCEPTANCE SHAPE (t11 inverted): a loop skeleton — a label plus a
/// branch back to it — lives in ONE `asm{}` block, with the varying body a
/// label-free `{ hole() }` splice. Instantiated TWICE in one proc, the two
/// `.top` labels must NOT collide (per-instantiation hygiene) and each `dbf`
/// must resolve to its OWN `.top`. This is exactly what `++`-concatenation
/// could not do (label unresolved across fragments).
#[test]
fn two_instantiations_of_a_label_template_do_not_collide() {
    let src = "\
module m
comptime fn hole() -> Code { return asm { nop } }
comptime fn loop_tmpl() -> Code {
    return asm {
    .top:
        { hole() }
        dbf     d0, .top
    }
}
proc p() {
    loop_tmpl()
    loop_tmpl()
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // Per instantiation: nop (4E71) ; dbf d0,.top (51C8 FFFC — disp back to
    // .top at the nop, -4 from the disp field). Two of them, then rts (4E75).
    assert_eq!(
        proc_bytes(&module, "text", "p", 14),
        vec![
            0x4E, 0x71, 0x51, 0xC8, 0xFF, 0xFC, // instantiation 1
            0x4E, 0x71, 0x51, 0xC8, 0xFF, 0xFC, // instantiation 2 — own .top
            0x4E, 0x75,
        ],
        "two label-carrying template instantiations must not collide and each dbf resolves locally"
    );
}

/// A spliced fragment may itself contain a splice — it's just evaluation.
#[test]
fn splice_inside_a_spliced_fragment() {
    let src = "\
module m
comptime fn inner() -> Code { return asm { nop } }
comptime fn middle() -> Code {
    return asm {
        move.w  #1, d0
        { inner() }
    }
}
comptime fn outer() -> Code {
    return asm {
        { middle() }
        rts
    }
}
proc p() {
    outer()
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // move.w #1,d0 ; nop ; rts
    assert_eq!(
        proc_bytes(&module, "text", "p", 8),
        vec![0x30, 0x3C, 0x00, 0x01, 0x4E, 0x71, 0x4E, 0x75],
        "a splice nested inside a spliced fragment must flatten through"
    );
}

/// A splice whose expr yields a non-Code value is a loud error naming Code.
#[test]
fn non_code_splice_is_an_error() {
    let src = "\
module m
comptime fn num() -> int { return 5 }
comptime fn wrap() -> Code {
    return asm {
        { num() }
        rts
    }
}
proc p() {
    wrap()
}
";
    let (_, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("splice must evaluate to Code") && e.contains("int")),
        "a non-Code splice must error naming Code and the actual type: {errs:?}"
    );
}

/// A splice whose expr yields a `Data` value gets the steering error (data
/// belongs in `dc`/`bytes()`), distinct from the generic non-Code message.
#[test]
fn data_splice_is_a_steering_error() {
    let src = "\
module m
comptime fn blob() -> Data { return bytes(\"AB\") }
comptime fn wrap() -> Code {
    return asm {
        { blob() }
        rts
    }
}
proc p() {
    wrap()
}
";
    let (_, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("data belongs in") && e.contains("dc")),
        "a Data splice must get the steering error: {errs:?}"
    );
}

/// PROBE (spec §4): a `{...}` at PROC-BODY statement position (outside an
/// `asm{}` template, where `splices_allowed` is false) is NOT a splice — it
/// falls through to instruction parsing and is a clean parse error. This
/// documents the "asm-block-only v1" boundary the spec accepts.
#[test]
fn splice_outside_asm_template_is_not_recognized() {
    let src = "\
module m
comptime fn ins() -> Code { return asm { nop } }
proc p() {
    { ins() }
    rts
}
";
    let (file, perrs) = parse_str(src);
    let _ = file;
    assert!(
        perrs.iter().any(|d| d.level == Level::Error),
        "a `{{...}}` at proc-body position (splices not allowed) must be a parse error"
    );
}

/// The aabb.emp fix pattern: a helper returns a Reg-move or empty by comparing
/// Reg params, spliced into the stream. Distinct registers emit the move;
/// aliased registers splice nothing. (Formerly an if-branch `asm{}` that
/// yielded Unit → the move was latently dead.)
#[test]
fn conditionally_empty_reg_splice_emits_only_when_distinct() {
    let src = "\
module m
comptime fn lead(a: Reg, b: Reg) -> Code {
    if a != b { return asm { move.w {a}, {b} } }
    return asm { }
}
comptime fn distinct() -> Code {
    return asm {
        { lead(d0, d1) }
        rts
    }
}
comptime fn alias() -> Code {
    return asm {
        { lead(d0, d0) }
        rts
    }
}
proc pd() {
    distinct()
}
proc pa() {
    alias()
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // distinct: move.w d0,d1 (3200) ; rts (4E75)
    assert_eq!(proc_bytes(&module, "text", "pd", 4), vec![0x32, 0x00, 0x4E, 0x75]);
    // aliased: the splice inlines nothing ; rts only
    assert_eq!(proc_bytes(&module, "text", "pa", 2), vec![0x4E, 0x75]);
}
