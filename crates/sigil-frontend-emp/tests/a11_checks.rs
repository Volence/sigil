//! A11: what the checks say about the instructions A11 makes writable. The
//! bytes are pinned in `sigil-harness/tests/emp_a11_golden.rs`; this file pins
//! that each new instruction is SEEN by the contract checks, not merely encoded.
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Level;

/// Every diagnostic of a one-proc module, as `(is_error, message)`.
fn diags(header: &str, body: &str) -> Vec<(bool, String)> {
    module_diags(&format!("module m\nproc P () {header} {{\n{body}\n}}\n"))
}

/// Every diagnostic of a whole module source, as `(is_error, message)`.
fn module_diags(src: &str) -> Vec<(bool, String)> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let opts = LowerOptions { roots: None, initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let (_m, d) = lower_module(&file, &opts);
    d.iter().map(|d| (d.level == Level::Error, d.message.clone())).collect()
}

fn has(d: &[(bool, String)], id: &str) -> bool {
    d.iter().any(|(_, m)| m.starts_with(id))
}

#[test]
fn the_new_arithmetic_writes_its_destination_register() {
    let d = diags("clobbers()", "    abcd d0, d1\n    rts");
    assert!(has(&d, "[proc.clobber-undeclared] `P` writes `d1`"), "{d:?}");
    for line in ["abcd d0, d1", "sbcd d0, d1", "nbcd d1", "negx.w d1", "subx.w d0, d1"] {
        let d = diags("clobbers(d1)", &format!("    {line}\n    rts"));
        assert_eq!(d, [], "`{line}` under clobbers(d1)");
    }
    let d = diags("clobbers(d0)", "    exg d0, d1\n    rts");
    assert!(has(&d, "[proc.clobber-undeclared] `P` writes `d1`"), "{d:?}");
}

#[test]
fn rtr_ends_a_proc_and_pops_the_saved_ccr_word() {
    // The idiom: save the condition codes, work, return restoring them.
    let d = diags("clobbers()", "    move.w sr, -(sp)\n    rtr");
    assert_eq!(d, [], "a balanced `rtr` body");
    // A leftover long under the saved word is still an imbalance.
    let d = diags("clobbers()", "    move.l d0, -(sp)\n    move.w sr, -(sp)\n    rtr");
    assert!(has(&d, "[stack.unbalanced]"), "{d:?}");
}
