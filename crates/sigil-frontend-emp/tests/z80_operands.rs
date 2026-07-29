//! T1 — the Z80 operand model end-to-end: `.emp` Z80 source → image bytes,
//! checked against the asl-golden bytes (`sigil-isa`'s `z80.rs` +
//! `z80_golden_vectors.txt`) for the same instruction. These are FRONTEND tests
//! — the mapper (`eval/asm.rs`) + the Z80 lowering (`lower/code.rs`) — not new
//! ISA facts. The design note is
//! `docs/superpowers/notes/2026-07-28-z80-t1-operand-model.md`.
//!
//! Every code fragment rides in a `proc` inside a `(cpu: z80, vma: $0)` section:
//! a proc is the vehicle whose body runs the SAME `eval_asm` → `lower_code_buf`
//! path a raw `asm { }` block uses, so the operand model is exercised
//! identically. Each proc ends in `ret` (`0xC9`) — a Z80 terminator, so no
//! `[proc.undeclared-fallthrough]` — and the tests pin the leading bytes.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

/// Parse + lower `src` for the 68k default (a Z80 section opts in explicitly),
/// asserting a clean parse. Returns the module and the lowering diagnostics.
fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    )
}

/// The linked bytes of a named section.
fn section_bytes(module: &Module, name: &str) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

/// Wrap one Z80 body line in a `(cpu: z80, vma: $0)` section proc `P`, lower it,
/// and return the section's linked bytes (the proc body + the trailing `ret`).
fn z80_body(body: &str) -> Vec<u8> {
    let src = format!(
        "module m\nsection s (cpu: z80, vma: $0) {{\n  proc P() {{\n    {body}\n    ret\n  }}\n}}\n"
    );
    let (module, diags) = lower(&src);
    assert!(diags.is_empty(), "lower diagnostics for `{body}`: {diags:?}");
    section_bytes(&module, "s")
}

// ---- item 1: the probe -----------------------------------------------------

#[test]
fn probe_ld_a_imm8() {
    // `ld a, 5` = 3E 05 (z80.rs golden `ld r,n`), then `ret` = C9.
    assert_eq!(z80_body("ld a, 5"), vec![0x3E, 0x05, 0xC9]);
}
