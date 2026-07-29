//! t27 phase-1(c) — the resident-Z80-address 16-bit VALUE cell.
//!
//! `seq_opcode_tab.asm` is 32 × `dw <resident-Z80-label>` in the banked
//! `$8000`-window head: each cell is the RESIDENT phase-0 Z80 address of a
//! `Seq_Op_*` handler, written little-endian, read through the window as data.
//! Before t27 a `dc.w <label>` in a Z80 section produced a `Cell::SymRef` →
//! `[cross-cpu.unwindowed-pointer]` (the guard assumes any Z80-section pointer is
//! a 68k address needing `winptr`). But a resident Z80 address is neither a 68k
//! address nor a windowed pointer — it is a raw 16-bit Z80 address.
//!
//! Implements the split the `lower/data.rs` comment anticipated: split the
//! `(Z80, _, false)` `fixup_kind` arm by WIDTH — a width-2 Z80-local reference
//! reuses `Value16Le` (the code's own naming call: "an Abs16Le … no new kind
//! warranted"). `Value16Le`'s unsigned u16 window check catches a genuine 68k
//! address (> $FFFF) that reaches here without `winptr` as a LINK-time
//! `[value.out-of-range]` error — so the guard's protection survives; and a
//! width-4 pointer keeps the lower-time `[cross-cpu.unwindowed-pointer]`.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Fixup, FixupKind, Fragment, Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    )
}

fn fixups_of(module: &Module, name: &str) -> Vec<Fixup> {
    module
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section `{name}`"))
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(d.fixups.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Link `module` and return either the named section's bytes or the link
/// diagnostics (for the negative control).
fn link_bytes(module: &Module, name: &str) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)?;
    let linked = sigil_link::link(&resolved, &SymbolTable::new())?;
    Ok(linked.section(name).expect("linked section").bytes.clone())
}

/// A banked `(cpu:z80, vma:$8000)` section holding one `dc.w <ResLabel>` cell,
/// plus a "resident" section placing `ResLabel` at `res_vma`.
fn src_with_resident(res_vma: u32) -> String {
    format!(
        "module m\n\
         section tab (cpu: z80, vma: $8000) {{\n\
           proc SeqOpcodeTable() clobbers() {{\n\
             dc.w ResLabel\n\
           }}\n\
         }}\n\
         section resident (cpu: z80, vma: ${res_vma:X}) {{\n\
           data ResLabel: u8 = $00\n\
         }}\n"
    )
}

/// FRAGMENT: a `dc.w <resident-label>` in a Z80 section defers to ONE Value16Le
/// fixup (width 2), NOT the cross-cpu error.
#[test]
fn resident_dcw_defers_value16le() {
    let (module, diags) = lower(&src_with_resident(0x0123));
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "a resident dc.w must NOT error at lower: {diags:?}"
    );
    let fixups = fixups_of(&module, "tab");
    assert_eq!(fixups.len(), 1, "one value fixup, got: {fixups:?}");
    assert_eq!(fixups[0].kind, FixupKind::Value16Le);
    assert_eq!(fixups[0].kind.byte_width(), 2);
}

/// POSITIVE CONTROL / P1 (t24 undoctored side): a resident label at $0123
/// (≤ $FFFF) links to the little-endian bytes `23 01`.
#[test]
fn resident_dcw_links_little_endian() {
    let (module, diags) = lower(&src_with_resident(0x0123));
    assert!(diags.iter().all(|d| d.level != Level::Error), "lower: {diags:?}");
    assert_eq!(link_bytes(&module, "tab").expect("links"), vec![0x23, 0x01]);
}

/// NEGATIVE CONTROL (t24 doctored side): a symbol resolving to $58000 (a 68k-scale
/// address, > $FFFF) referenced via `dc.w` in a Z80 section is a LINK-time
/// `[value.out-of-range]` error — the guard's protection survives at width 2, so
/// the split never silently truncates a 68k address into the window.
#[test]
fn resident_dcw_68k_address_is_out_of_range() {
    let (module, diags) = lower(&src_with_resident(0x58000));
    assert!(diags.iter().all(|d| d.level != Level::Error), "lower should be clean: {diags:?}");
    let err = link_bytes(&module, "tab").expect_err("a >$FFFF address must fail the u16 window");
    assert!(
        err.iter().any(|d| d.message.contains("[value.out-of-range]")),
        "expected [value.out-of-range] for a 68k address at width 2, got: {err:?}"
    );
}

/// GUARD SURVIVES (width 4): a `dc.l <label>` (width-4 pointer) in a Z80 section
/// still errors `[cross-cpu.unwindowed-pointer]` — the split is width-2-only; a
/// 4-byte 68k pointer is still the un-windowed-pointer error at lower time.
#[test]
fn dcl_width4_in_z80_still_unwindowed_pointer_error() {
    let src = "module m\n\
               section tab (cpu: z80, vma: $8000) {\n\
                 proc T() clobbers() {\n\
                   dc.l ResLabel\n\
                 }\n\
               }\n\
               section resident (cpu: z80, vma: $0123) {\n\
                 data ResLabel: u8 = $00\n\
               }\n";
    let (_module, diags) = lower(src);
    assert!(
        diags.iter().any(|d| d.level == Level::Error
            && d.message.contains("[cross-cpu.unwindowed-pointer]")),
        "a width-4 Z80 pointer must stay the unwindowed-pointer error, got: {diags:?}"
    );
}
