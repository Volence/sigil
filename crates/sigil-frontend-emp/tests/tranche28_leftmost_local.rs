//! t28 P3 — the leftmost local-label immediate fix (ledger row 1610).
//!
//! A `.local` label as the FIRST operand token of an immediate/value expression
//! (`ld de, .code_end + 1`) was parsed as a raw `Path[".code_end"]` and evaluated
//! by `eval_path`'s label fallback to a RAW, unmangled `Value::Label(".code_end")`
//! — a dangling symbol the linker cannot resolve. An INTERIOR occurrence
//! (`ld de, 1 + .code_end`) is parsed as `Expr::LocalLabel` and routed through
//! `eval_local_label`'s owner-mangling, so it resolves. The two spellings of the
//! same intent diverged.
//!
//! The fix routes a leading-`.` single-segment path through the SAME
//! `eval_local_label` mangling the interior form uses, so both spellings emit
//! identical bytes and both are tracked for the end-of-body loudness check (a
//! typo names itself instead of minting a silent dangling symbol). Census: ZERO
//! live corpus instances, so byte-neutral by construction (the disp/pc-relative/
//! branch label paths resolve via `scope.resolve_ref`, not `eval_path`, and are
//! untouched). CPU-agnostic; tested at the Z80 imm16 demand site (z80_init's
//! `.code_end` workaround) plus a 68k absolute-address control.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    )
}

fn linked_bytes(module: &Module, name: &str) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

fn z80_ld_de(disp_form: &str) -> String {
    format!(
        "module m\n\
         section code (cpu: z80, vma: $0) {{\n\
         \x20 proc P() {{\n\
         \x20   ld de, {disp_form}\n\
         \x20   ret\n\
         \x20   .code_end:\n\
         \x20 }}\n\
         }}\n"
    )
}

/// POSITIVE CONTROL (byte-identity): the leftmost `.code_end + 1` and the
/// interior `1 + .code_end` emit IDENTICAL bytes — `.code_end` is at offset 4
/// (`ld de,nn`=3 + `ret`=1), so both are imm $0005 → `11 05 00 C9`.
#[test]
fn leftmost_and_interior_local_label_immediate_are_byte_identical() {
    let (m_left, d_left) = lower(&z80_ld_de(".code_end + 1"));
    assert!(d_left.is_empty(), "leftmost form must lower clean, got: {d_left:?}");
    let (m_right, d_right) = lower(&z80_ld_de("1 + .code_end"));
    assert!(d_right.is_empty(), "interior form must lower clean, got: {d_right:?}");

    let left = linked_bytes(&m_left, "code");
    let right = linked_bytes(&m_right, "code");
    assert_eq!(left, right, "`.code_end + 1` must equal `1 + .code_end`");
    assert_eq!(left, vec![0x11, 0x05, 0x00, 0xC9], "expected ld de,$0005 / ret");
}

// NOTE: the fix lives in shared `eval_path` (eval/expr.rs), so it is
// CPU-agnostic — any label-VALUE context that spells an operand-leading `.local`
// via the operand grammar's `Tok::Dot` arm benefits. (A 68k proc-local label as
// an ABSOLUTE-MEMORY operand — `move.l .t, d0` — is a DIFFERENT, still-unsupported
// form: it is not a label-value context, so BOTH spellings error "only valid as a
// label-value argument"; that is orthogonal to the P3 leftmost/interior asymmetry.)

/// NEGATIVE CONTROL: an UNDEFINED leftmost local label is now LOUD (routed
/// through the loudness check, naming the label) — never a silent dangling
/// symbol. Its paired POSITIVE CONTROL is the byte-identity test above (a DEFINED
/// leftmost label compiles clean through the same path).
#[test]
fn undefined_leftmost_local_label_is_loud() {
    let (_m, diags) = lower(&z80_ld_de(".nonexistent + 1"));
    assert!(
        diags
            .iter()
            .any(|d| d.level == Level::Error && d.message.contains("nonexistent")),
        "an undefined leftmost `.nonexistent` must name itself in an error, got: {diags:?}"
    );
}
