//! Spliced INDEX register in an `asm{}`-template indexed EA (`d(An,{Xn}[.size])`).
//!
//! The base register of `d(An,Xn)` already accepts an evaluated/spliced value
//! (via `ind_single_reg`), but the INDEX slot only matched a literal register
//! path — so a comptime-fn helper that emits `move.w DISP({base},{off}), dst`
//! with `off: Reg` was rejected ("indexed addressing needs a valid index
//! register"). And a size suffix after a spliced index (`{off}.w`) failed to
//! parse ("expected `)`, found Dot"). These tests close the base/index
//! asymmetry — the frame_piece_count helper is the first consumer.

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

/// A spliced index register (no explicit size → default `.w`) resolves via eval
/// exactly like the spliced base, emitting the brief-extension indexed EA.
/// `move.w 4(a3,d3.w),d0` = `30 33 30 04` (index d3, word, disp 4).
#[test]
fn spliced_index_register_resolves_via_eval() {
    let src = "\
module m
comptime fn idx(base: Reg, off: Reg, dst: Reg) -> Code {
    return asm { move.w 4({base}, {off}), {dst} }
}
proc p() {
    idx(a3, d3, d0)
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    assert_eq!(
        proc_bytes(&module, "text", "p", 6),
        vec![0x30, 0x33, 0x30, 0x04, 0x4E, 0x75],
        "spliced index reg must emit `move.w 4(a3,d3.w),d0` then rts"
    );
}

/// A `.w` size suffix directly after a spliced index register parses and emits
/// the word-index brief extension — same bytes as the default (`30 33 30 04`).
#[test]
fn word_size_suffix_after_spliced_index_parses() {
    let src = "\
module m
comptime fn idx(base: Reg, off: Reg, dst: Reg) -> Code {
    return asm { move.w 4({base}, {off}.w), {dst} }
}
proc p() {
    idx(a3, d3, d0)
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    assert_eq!(
        proc_bytes(&module, "text", "p", 6),
        vec![0x30, 0x33, 0x30, 0x04, 0x4E, 0x75],
        "`{{off}}.w` must parse and emit the word-index EA"
    );
}

/// A `.l` size suffix after a spliced index selects the long-index brief
/// extension: bit 11 set → `30 33 38 04` (vs `30 33 30 04` for word).
#[test]
fn long_size_suffix_after_spliced_index_selects_long_index() {
    let src = "\
module m
comptime fn idx(base: Reg, off: Reg, dst: Reg) -> Code {
    return asm { move.w 4({base}, {off}.l), {dst} }
}
proc p() {
    idx(a3, d3, d0)
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    assert_eq!(
        proc_bytes(&module, "text", "p", 6),
        vec![0x30, 0x33, 0x38, 0x04, 0x4E, 0x75],
        "`{{off}}.l` must select the long-index brief extension"
    );
}

/// A non-register in the index slot (a const int reached via eval) still errors
/// cleanly — the eval fallback must not panic or silently accept a bad index.
#[test]
fn non_register_index_errors_cleanly() {
    let src = "\
module m
const SOME = 5
comptime fn idx(base: Reg, dst: Reg) -> Code {
    return asm { move.w 4({base}, SOME), {dst} }
}
proc p() {
    idx(a3, d0)
    rts
}
";
    let (_module, diags) = lower(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("needs a valid index register")),
        "a non-register index must error cleanly: {errs:?}"
    );
}

/// The exact frame_piece_count consumer shape: a NAMED const displacement with a
/// spliced base AND spliced `.w` index — `move.w DISP(a3,d3.w),d3`. DISP=4 here
/// (the real `FRAME_PIECE_COUNT`), dst==index==d3 as in load_object.
#[test]
fn symbolic_disp_with_spliced_base_and_index() {
    let src = "\
module m
const FRAME_PIECE_COUNT = 4
comptime fn frame_piece_count(base: Reg, off: Reg, dst: Reg) -> Code {
    return asm { move.w FRAME_PIECE_COUNT({base}, {off}.w), {dst} }
}
proc p() {
    frame_piece_count(a3, d3, d3)
    rts
}
";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "unexpected errors: {:?}", errors(&diags));
    // move.w 4(a3,d3.w),d3 = 36 33 30 04 ; rts = 4E75
    assert_eq!(
        proc_bytes(&module, "text", "p", 6),
        vec![0x36, 0x33, 0x30, 0x04, 0x4E, 0x75],
        "the frame_piece_count consumer shape must emit move.w 4(a3,d3.w),d3"
    );
}
