//! Tranche 20 — `CodeOperand::DispSymInd` (symbolic d16 displacement over An,
//! the `jmp .jump_table(a1)` jump-table dispatch idiom) — the demanded-feature
//! negative probes. Byte-parity positives live in
//! `sigil-cli/tests/tranche20_spelling_probes.rs` (P4/P5, vs the AS front-end).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Level;

fn lower_errors(emp: &str) -> Vec<String> {
    let (file, pdiags) = parse_str(emp);
    assert!(
        !pdiags.iter().any(|d| d.level == Level::Error),
        "parse errors: {:?}",
        pdiags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (_module, ldiags) = lower_module(&file, &opts);
    ldiags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

/// A data-register base is not a 68k `(d16,An)` EA — loud, naming the class.
#[test]
fn data_register_base_is_rejected() {
    let errs = lower_errors(
        "module m\npub proc P () {\n        jmp     .t(d1)\n    .t:\n        rts\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("[lower.disp-sym-operand]") && e.contains("ADDRESS")),
        "expected the address-register reject, got: {errs:?}"
    );
}

/// A typo'd local label in displacement position stays LOUD — the end-of-body
/// label check or the link rejects it; it must not silently become a zero disp.
/// (The label scope maps only defined labels; an unknown `.name` displacement
/// resolves to a hidden symbol the link cannot define.)
#[test]
fn unknown_local_label_disp_fails_at_link() {
    let src = "module m\npub proc P () {\n        jmp     .missing(a1)\n        rts\n}\n";
    let (file, pdiags) = parse_str(src);
    assert!(!pdiags.iter().any(|d| d.level == Level::Error), "parse: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    let lower_ok = !ldiags.iter().any(|d| d.level == Level::Error);
    if lower_ok {
        // Not caught at lower — the link MUST reject the unresolvable hidden
        // symbol loudly (never a silent zero displacement).
        let empty = sigil_ir::SymbolTable::new();
        let resolved = sigil_link::resolve_layout(&module.sections, &empty, true);
        let linked = resolved.and_then(|r| sigil_link::link(&r, &empty));
        assert!(linked.is_err(), "an undefined `.missing` displacement must fail the link");
    }
}

/// The d16 field is one word — mixing a symbolic displacement with a symbolic
/// absolute operand in one instruction is fenced loudly (no offset proof).
#[test]
fn mixing_with_symbolic_absolute_is_rejected() {
    let errs = lower_errors(
        "module m\npub proc P () {\n        move.w  .t(a1), Ext_Sym\n    .t:\n        rts\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("[lower.disp-sym-operand]")),
        "expected the mixing reject, got: {errs:?}"
    );
}
