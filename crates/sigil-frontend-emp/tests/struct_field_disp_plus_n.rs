//! WATCH SITE (shared-struct-module micro-batch, row 1051; Fable item-4 probe).
//!
//! entity_window.emp:845/1642 read `move.b Act_grid_w+1(a2), d1` — the LOW byte
//! ($05) of the word field `grid_w` ($04). When the file-local offset const
//! `Act_grid_w` unwinds to the shared `struct Act`, that site needs a field
//! displacement PLUS ONE — the corpus's ONLY field+N byte access into a word
//! field. This file is the persistent record of the item-4 composition probe.
//!
//! FINDING (2026-07-16): the NATURAL spelling `Act.grid_w + 1(a2)` does NOT
//! compose — the parser reads `Act.grid_w` as a bare symbol inside the arithmetic
//! displacement expression, so it never becomes a struct-field offset ("unknown
//! name `Act.grid_w`"). The `.field`-access sugar only special-cases a BARE
//! `Struct.field(An)` displacement (see `bare_field_disp_is_the_offset_control`),
//! not one wrapped in arithmetic. The parenthesized `(Struct.field + 1)(An)` form
//! does not parse (`(disp)(An)` is not a displacement grammar).
//!
//! BYTE-NEUTRAL FALLBACK (the shipped path): `offsetof(Act, grid_w) + 1(a2)`
//! composes to the identical bytes `12 2A 00 05` (disp 5) — offsetof folds to a
//! comptime int that arithmetic accepts in the displacement slot, exactly like
//! the shipped `Act_grid_w + 1(a2)` const form. `bare_field_disp_...` proves the
//! plain field-disp mechanism, so every OTHER (no-+N) Sec/Act access in the
//! unwind is clean.
//!
//! LANGUAGE ASK (ledgered, NOT built here — step-4 verb (c), ask not stopgap):
//! `.field` access should compose inside displacement arithmetic (`Struct.field
//! + N(An)`) the way `offsetof(Struct, field) + N(An)` already does. When that
//! lands, `natural_field_plus_n_does_not_compose_today` flips red — the signal to
//! respell the two sites and delete this note.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn lower(src: &str) -> (sigil_ir::Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    )
}

fn section<'a>(module: &'a sigil_ir::Module, name: &str) -> &'a Section {
    module.sections.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("no section `{name}`"))
}

fn label_offset(sec: &Section, name: &str) -> u32 {
    sec.labels.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("no label `{name}`")).offset
}

fn linked_section_bytes(module: &sigil_ir::Module, name: &str) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

fn errors(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.as_str()).collect()
}

fn proc_bytes(module: &sigil_ir::Module, name: &str, len: usize) -> Vec<u8> {
    let s = section(module, "text");
    let off = label_offset(s, name) as usize;
    let linked = linked_section_bytes(module, "text");
    linked[off..off + len].to_vec()
}

/// `struct Act` prefix matching structs.asm: `sec_grid_ptr: *u8` ($00, 4 bytes),
/// `grid_w: u16` ($04). So offsetof(Act, grid_w) = $04 and +1 = $05 (the low byte).
const ACT: &str = "struct Act { sec_grid_ptr: *u8, grid_w: u16, grid_h: u16 }";

/// Lower `move.b <disp>(a2), d1` and return its 4 bytes.
fn disp_bytes(disp_form: &str) -> Result<Vec<u8>, Vec<String>> {
    let src = format!("module m\n{ACT}\nproc read() {{\n    move.b {disp_form}(a2), d1\n    rts\n}}\n");
    let (file, perrs) = parse_str(&src);
    if !perrs.is_empty() {
        return Err(perrs.iter().map(|d| d.message.clone()).collect());
    }
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    let errs = errors(&diags);
    if !errs.is_empty() {
        return Err(errs.iter().map(|s| s.to_string()).collect());
    }
    Ok(proc_bytes(&module, "read", 4))
}

/// The reference encoding: `move.b 5(a2), d1` = `12 2A 00 05`.
fn literal_disp5() -> Vec<u8> {
    disp_bytes("5").expect("literal disp control")
}

#[test]
fn offsetof_plus_n_fallback_composes() {
    // THE SHIPPED PATH: `offsetof(Act, grid_w) + 1(a2)` folds to disp 5, byte-
    // identical to the literal `move.b 5(a2), d1` and to the shipped const form.
    let bytes = disp_bytes("offsetof(Act, grid_w) + 1").expect("offsetof+N must compose");
    assert_eq!(bytes, literal_disp5(), "offsetof+N displacement must equal `move.b 5(a2), d1`");
    assert_eq!(bytes, vec![0x12, 0x2A, 0x00, 0x05], "expected move.b 5(a2),d1 encoding");
}

#[test]
fn natural_field_plus_n_does_not_compose_today() {
    // The gap this watch site records. If the language gains `.field`-in-disp
    // arithmetic, this flips red — respell the two sites and retire the note.
    let r = disp_bytes("Act.grid_w + 1");
    assert!(
        r.is_err() && r.as_ref().unwrap_err().iter().any(|e| e.contains("Act.grid_w")),
        "expected `Act.grid_w + 1(a2)` to fail with an unknown-name diagnostic; got {r:?}"
    );
}

#[test]
fn bare_field_disp_is_the_offset_control() {
    // Control: the bare field disp (no +N) folds to offsetof = 4 — proves the
    // field-disp mechanism works, isolating the `+ N` composition question.
    assert_eq!(disp_bytes("Act.grid_w").expect("bare field disp"), disp_bytes("4").unwrap());
}
