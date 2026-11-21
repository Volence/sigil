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

#[test]
fn usp_is_an_operand_of_the_two_moves_only() {
    assert_eq!(diags("clobbers()", "    move.l a0, usp\n    rts"), []);
    let d = diags("clobbers()", "    move.l usp, a1\n    rts");
    assert!(has(&d, "[proc.clobber-undeclared] `P` writes `a1`"), "{d:?}");
    for line in ["move.l usp, d0", "lea usp, a0", "move.w a0, usp", "move.l #1, usp", "tst.l usp"] {
        let d = diags("clobbers(d0, a0)", &format!("    {line}\n    rts"));
        assert!(errors(&d).iter().any(|m| m.starts_with("[operand.usp-unsupported] ")), "`{line}`: {d:?}");
    }
}

/// The error-level messages only.
fn errors(d: &[(bool, String)]) -> Vec<&str> {
    d.iter().filter(|(e, _)| *e).map(|(_, m)| m.as_str()).collect()
}

/// `move <ea>, ccr` and `eori #, ccr` write the condition codes only, not the
/// interrupt mask: `[proc.sr-undeclared]` stays silent where the same write to
/// `sr` fires, while the CCR-half contract checks still see the write.
#[test]
fn the_ccr_writes_are_flag_writes_not_mask_writes() {
    let d = diags("clobbers()", "    move.w d0, sr\n    rts");
    assert!(has(&d, "[proc.sr-undeclared] `P` writes `sr`"), "{d:?}");
    for line in ["move.w d0, ccr", "move.w #$1F, ccr", "eori.b #$01, ccr"] {
        assert_eq!(diags("clobbers()", &format!("    {line}\n    rts")), [], "`{line}` under clobbers()");
        let d = diags("preserves(sr.ccr)", &format!("    {line}\n    rts"));
        assert!(errors(&d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)`")), "`{line}`: {d:?}");
        let d = diags("preserves(sr)", &format!("    {line}\n    rts"));
        assert!(has(&d, "[proc.ccr-advisory] `P` declares bare `preserves(sr)`"), "`{line}`: {d:?}");
        // Bracketed by the save/restore pair, the condition codes round-trip.
        let body = format!("    move.w sr, -(sp)\n    {line}\n    move.w (sp)+, sr\n    rts");
        assert_eq!(diags("preserves(sr)", &body), [], "`{line}` bracketed");
    }
}

/// A `movem` load from a PC-relative source writes every register in its list.
#[test]
fn movem_from_a_pc_relative_source_writes_its_list() {
    let d = diags("clobbers(d0)", ".t:\n    movem.w .t(pc), d0-d1\n    rts");
    assert!(has(&d, "[proc.clobber-undeclared] `P` writes `d1`"), "{d:?}");
    assert!(!has(&d, "[proc.clobber-undeclared] `P` writes `d0`"), "{d:?}");
    assert_eq!(diags("clobbers(d0-d1)", ".t:\n    movem.l .t(pc,d0.w), d0-d1\n    rts"), []);
}
