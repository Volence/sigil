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

/// `rtr` replaces the condition codes with the saved word: the flags computed
/// before it say nothing about the return. The reviewer's two probes must read
/// the same, because at the return both have flags the checker cannot see.
#[test]
fn rtr_replaces_the_condition_codes_a_conditional_out_reads() {
    let header = "clobbers(d1) out(d0 if eq)";
    let probe = |third: &str| diags(header, &format!("    move.w sr, -(sp)\n    moveq #5, d0\n    {third}\n    rtr"));
    let known_ne = probe("moveq #1, d1");
    let unknown = probe("tst.w d1");
    assert_eq!(unknown, [], "flags unknown before `rtr`");
    assert_eq!(known_ne, unknown, "a `moveq` before `rtr` must not classify the return");
}

/// `rtr` IS the restore of a `move.w sr, -(sp)` save: it pops that word back
/// into the condition codes. The idiom keeps `preserves(sr.ccr)` and bare
/// `preserves(sr)`; a body that saves and drops the word unrestored does not.
#[test]
fn rtr_restores_the_saved_condition_codes() {
    let idiom = "    move.w sr, -(sp)\n    moveq #1, d1\n    rtr";
    assert_eq!(diags("clobbers(d1) preserves(sr.ccr)", idiom), [], "the `rtr` idiom under preserves(sr.ccr)");
    assert_eq!(diags("clobbers(d1) preserves(sr)", idiom), [], "the `rtr` idiom under preserves(sr)");
    // Control: the saved word is discarded and the proc returns unrestored.
    let dropped = "    move.w sr, -(sp)\n    moveq #1, d1\n    addq.l #2, sp\n    rts";
    let d = diags("clobbers(d1) preserves(sr.ccr)", dropped);
    assert!(
        errors(&d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)`")
            && m.contains("`rts` returns between the save and the restore")),
        "{d:?}"
    );
    // Control: `rtr` with no save pops a word nobody saved.
    let d = diags("clobbers(d1) preserves(sr.ccr)", "    moveq #1, d1\n    rtr");
    assert!(errors(&d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)`")), "{d:?}");
}

/// Each of the five extended / decimal instructions is SEEN writing its
/// destination: under `clobbers()` every one is charged with `d1`.
#[test]
fn each_new_arithmetic_word_is_charged_with_its_destination() {
    for line in ["abcd d0, d1", "sbcd d0, d1", "nbcd d1", "negx.w d1", "subx.w d0, d1"] {
        let d = diags("clobbers()", &format!("    {line}\n    rts"));
        assert!(has(&d, "[proc.clobber-undeclared] `P` writes `d1`"), "`{line}`: {d:?}");
    }
}

/// The `[call.flag-result-unused]` firings for the first proc of `src`, with
/// `Queue` declared to return `flag` (`carry` or `zero`).
fn flag_unused(src: &str, flag: &str) -> Vec<sigil_frontend_emp::flag_check::FlagFiring> {
    use sigil_frontend_emp::ast::Item;
    use std::collections::{BTreeMap, BTreeSet};
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let p = file.items.iter().find_map(|i| if let Item::Proc(p) = i { Some(p) } else { None }).expect("a proc");
    let (buf, _d, _n) = sigil_frontend_emp::eval::eval_proc_body(
        &file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[],
        &sigil_frontend_emp::contract::InterfaceEnv::empty(),
    );
    let fc = BTreeMap::from([("Queue".to_string(), BTreeSet::from([flag.to_string()]))]);
    sigil_frontend_emp::flag_check::check_flag_unused(&p.name, &buf.expect("codebuf").items, &fc, &[], Cpu::M68000)
}

/// MOVE USP: "Condition Codes: Not affected." A flag set before either `usp`
/// move is still the same flag after it, for every check that reads flags.
#[test]
fn a_flag_result_survives_a_usp_move() {
    // The flag-result walk: a carry (or zero) result read after a `usp` move is
    // still the callee's. Control: `move.l a0, d0` writes both flags.
    for (flag, branch) in [("carry", "bcs.s"), ("zero", "beq.s")] {
        for (mid, fires) in [("move.l a0, usp", false), ("move.l usp, a0", false), ("move.l a0, d0", true)] {
            let src = format!("module m\nproc P () clobbers(d0/a0) {{\n    jbsr Queue\n    {mid}\n    {branch} .x\n.x:\n    rts\n}}\n");
            let f = flag_unused(&src, flag);
            assert_eq!(!f.is_empty(), fires, "{flag} across `{mid}`: {f:?}");
        }
    }
}

/// The conditional-out SURVIVES half reads the flags at the return through a
/// `usp` move.
#[test]
fn a_conditional_out_reads_the_flags_through_a_usp_move() {
    // The conditional-out SURVIVES half reads the flags at the return: `moveq #1`
    // leaves Z clear, so the return is a `!eq` exit where `d0` must survive.
    // Across a `usp` move that is still known (it fires, as across `lea`);
    // across `move.l a0, d2` the flags are unknown and nothing can be said.
    let survives = |mid: &str| {
        let d = diags("clobbers(d1, d2, a0, a1) out(d0 if eq)", &format!("    moveq #5, d0\n    moveq #1, d1\n    {mid}\n    rts"));
        has(&d, "[proc.out-cond-survives-unverifiable] `P` declares `out(d0 if eq)`")
    };
    assert!(survives("lea (a0), a1"), "the transparent baseline");
    assert!(survives("move.l a0, usp"), "flags across `move.l a0, usp`");
    assert!(survives("move.l usp, a0"), "flags across `move.l usp, a0`");
    assert!(!survives("move.l a0, d2"), "control: `move.l a0, d2` writes the flags");
}

/// `preserves(sr.ccr)`: neither `usp` move touches the caller's condition codes.
#[test]
fn preserves_ccr_holds_across_a_usp_move() {
    assert_eq!(diags("preserves(sr.ccr)", "    move.l a0, usp\n    rts"), []);
    assert_eq!(diags("clobbers(a0) preserves(sr.ccr)", "    move.l usp, a0\n    rts"), []);
    let d = diags("clobbers(d0) preserves(sr.ccr)", "    move.l a0, d0\n    rts");
    assert!(errors(&d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)`")), "control: {d:?}");
}

#[test]
fn a_frame_is_balanced_only_when_it_is_closed() {
    let d = diags("clobbers() preserves(a6)", "    link a6, #-8\n    move.w d0, -4(a6)\n    unlk a6\n    rts");
    assert_eq!(d, [], "a paired frame that restores a6");
    let d = diags("clobbers(a6)", "    link a6, #-8\n    rts");
    assert_eq!(errors(&d).len(), 1, "{d:?}");
    assert!(errors(&d)[0].starts_with("[stack.unbalanced] in `P`: this path returns with 12 bytes"), "{d:?}");
    // `link` writes its frame register even though it is the FIRST operand.
    let d = diags("clobbers()", "    link a6, #-8\n    unlk a6\n    rts");
    let charged = d.iter().filter(|(_, m)| m.starts_with("[proc.clobber-undeclared] `P` writes `a6`")).count();
    assert_eq!(charged, 2, "one for `link`, one for `unlk`: {d:?}");
}

/// LINK / UNLK: "Condition Codes: Not affected." The flags computed before a
/// frame op are still the flags after it, for every check that reads flags.
#[test]
fn a_frame_op_leaves_the_condition_codes_alone() {
    // The conditional-out SURVIVES half: `moveq #1, d1` leaves Z clear, so the
    // return is a `!eq` exit where `d0` must survive, and it fires, provided the
    // flags are still known at the return. Control: `move.l a0, d2` writes them.
    let survives = |mid: &str| {
        let body = format!("    link a6, #-8\n    moveq #5, d0\n    moveq #1, d1\n    {mid}\n    rts");
        let d = diags("clobbers(d1, d2, a6) out(d0 if eq)", &body);
        has(&d, "[proc.out-cond-survives-unverifiable] `P` declares `out(d0 if eq)`")
    };
    assert!(survives("unlk a6"), "flags across `unlk a6`");
    assert!(!survives("move.l a0, d2\n    unlk a6"), "control: `move.l a0, d2` writes the flags");
    // `preserves(sr.ccr)`: a frame opened and closed does not touch the
    // caller's condition codes. Control: a flag write inside the frame does.
    assert_eq!(diags("clobbers(a6) preserves(sr.ccr)", "    link a6, #-8\n    unlk a6\n    rts"), []);
    let d = diags("clobbers(a6, d0) preserves(sr.ccr)", "    link a6, #-8\n    moveq #0, d0\n    unlk a6\n    rts");
    assert!(errors(&d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)`")), "control: {d:?}");
}

#[test]
fn every_sr_write_form_is_an_undeclared_mask_write() {
    for line in ["stop #$2000", "ori.w #$0700, sr", "andi.w #$F8FF, sr", "eori.w #$0700, sr"] {
        let d = diags("clobbers()", &format!("    {line}\n    rts"));
        assert!(has(&d, "[proc.sr-undeclared] `P` writes `sr`"), "`{line}`: {d:?}");
        let d = diags("clobbers(sr)", &format!("    {line}\n    rts"));
        assert!(!has(&d, "[proc.sr-undeclared]"), "`{line}` declared: {d:?}");
    }
    // A condition-code-only write is not a mask write.
    for line in ["eori.b #1, ccr", "move.w d0, ccr"] {
        let d = diags("clobbers()", &format!("    {line}\n    rts"));
        assert!(!has(&d, "[proc.sr-undeclared]"), "`{line}`: {d:?}");
    }
}

#[test]
fn stop_cannot_hide_behind_preserves_sr() {
    let d = diags("clobbers() preserves(sr)", "    stop #$2700\n    rts");
    assert!(has(&d, "[proc.preserves-sr-unbalanced]"), "{d:?}");
    let d = diags("clobbers() preserves(sr)", "    move.w sr, -(sp)\n    stop #$2000\n    move.w (sp)+, sr\n    rts");
    assert!(!has(&d, "[proc.preserves-sr-unbalanced]"), "{d:?}");
}

/// STOP: "Condition Codes: Set according to the immediate operand." ANDI / ORI /
/// EORI to SR change C and Z exactly as the low byte of the immediate says. A
/// callee's flag result read after one is the callee's only when the write left
/// that flag alone.
#[test]
fn the_sr_writes_reach_the_flags_as_their_immediate_says() {
    for (flag, branch) in [("carry", "bcs.s"), ("zero", "beq.s")] {
        for (mid, fires) in [
            ("stop #$2700", true),
            ("ori.w #$0700, sr", false),
            ("andi.w #$F8FF, sr", false),
            ("eori.w #$0700, sr", false),
            ("ori.w #$0705, sr", true),
            ("andi.w #$F8FA, sr", true),
        ] {
            let src = format!("module m\nproc P () clobbers(sr) {{\n    jbsr Queue\n    {mid}\n    {branch} .x\n.x:\n    rts\n}}\n");
            let f = flag_unused(&src, flag);
            assert_eq!(!f.is_empty(), fires, "{flag} across `{mid}`: {f:?}");
        }
    }
    // `preserves(sr.ccr)`: each one rewrites the condition codes outside a bracket.
    for line in ["stop #$2700", "ori.w #$0700, sr"] {
        let d = diags("clobbers(sr.mask) preserves(sr.ccr)", &format!("    {line}\n    rts"));
        assert!(errors(&d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)`")), "`{line}`: {d:?}");
    }
}

/// The `branch_const` firings for the first proc of `src`.
fn branch_const(src: &str) -> Vec<sigil_frontend_emp::branch_const::BranchConstFiring> {
    use sigil_frontend_emp::ast::Item;
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let p = file.items.iter().find_map(|i| if let Item::Proc(p) = i { Some(p) } else { None }).expect("a proc");
    let (buf, _d, _n) = sigil_frontend_emp::eval::eval_proc_body(
        &file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[],
        &sigil_frontend_emp::contract::InterfaceEnv::empty(),
    );
    sigil_frontend_emp::branch_const::check_branch_const(&p.name, &buf.expect("codebuf").items)
}

/// RESET pulses the external RESET line and changes no CPU register, flag or
/// stack word ("Condition Codes: Not affected."), and execution goes on with the
/// next instruction. Every check reads straight through it.
#[test]
fn reset_leaves_registers_flags_and_flow_alone() {
    // No register is charged, and it is not a terminator: a body that ends in
    // `reset` can still run off its end.
    assert_eq!(diags("clobbers()", "    reset\n    rts"), []);
    let d = diags("clobbers()", "    reset");
    assert!(has(&d, "[proc.undeclared-fallthrough] `P` can reach its closing `}`"), "{d:?}");
    // The flag-result walk: a callee's carry (or zero) read after `reset` is
    // still the callee's. Control: `move.l a0, d0` writes both flags.
    for (flag, branch) in [("carry", "bcs.s"), ("zero", "beq.s")] {
        for (mid, fires) in [("reset", false), ("move.l a0, d0", true)] {
            let src = format!("module m\nproc P () clobbers(d0) {{\n    jbsr Queue\n    {mid}\n    {branch} .x\n.x:\n    rts\n}}\n");
            let f = flag_unused(&src, flag);
            assert_eq!(!f.is_empty(), fires, "{flag} across `{mid}`: {f:?}");
        }
    }
    // The conditional-out SURVIVES half: `moveq #1, d1` leaves Z clear, so the
    // return is a `!eq` exit where `d0` must survive. It fires only while the
    // flags are still known at the return.
    let survives = |mid: &str| {
        let d = diags("clobbers(d1, d2) out(d0 if eq)", &format!("    moveq #5, d0\n    moveq #1, d1\n    {mid}\n    rts"));
        has(&d, "[proc.out-cond-survives-unverifiable] `P` declares `out(d0 if eq)`")
    };
    assert!(survives("reset"), "flags across `reset`");
    assert!(!survives("move.l a0, d2"), "control: `move.l a0, d2` writes the flags");
    // `preserves(sr.ccr)` holds across it. Control: a flag write does not.
    assert_eq!(diags("preserves(sr.ccr)", "    reset\n    rts"), []);
    let d = diags("clobbers(d0) preserves(sr.ccr)", "    reset\n    moveq #0, d0\n    rts");
    assert!(errors(&d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)`")), "control: {d:?}");
    // The constant-branch net: `moveq #0` sets Z, `reset` keeps it, so the
    // `beq` is always taken. Control: `tst.w d1` makes Z unknown.
    let src = |mid: &str| format!("module m\nproc P () clobbers(d0) {{\n    moveq #0, d0\n    {mid}\n    beq.s .x\n.x:\n    rts\n}}\n");
    let f = branch_const(&src("reset"));
    assert!(f.len() == 1 && f[0].cc == "eq" && f[0].always_taken, "{f:?}");
    assert_eq!(branch_const(&src("tst.w d1")), [], "control");
}

#[test]
fn chk_trapv_and_reset_are_plain_instructions_to_the_checks() {
    assert_eq!(diags("clobbers()", "    chk.w d1, d0\n    trapv\n    reset\n    rts"), []);
}

/// Review Focus 1: a new mnemonic does not take a name away from data.
#[test]
fn a_const_or_label_named_after_a_new_mnemonic_is_still_a_name() {
    let d = module_diags("module m\nconst stop = 3\nconst link = 4\nproc P () clobbers(d0) {\n.chk:\n    moveq #stop + link, d0\n    bra.s .chk\n}\n");
    assert_eq!(d, [], "{d:?}");
}

/// The conditional-out SURVIVES probe: `moveq #1, d1` leaves Z clear, so the
/// return is a `!eq` exit where `d0` must survive. It fires only while the
/// flags are still known at the return.
fn out_cond_survives(mid: &str) -> bool {
    let d = diags("clobbers(d1, d2, a1) out(d0 if eq)", &format!("    moveq #5, d0\n    moveq #1, d1\n    {mid}\n    rts"));
    has(&d, "[proc.out-cond-survives-unverifiable] `P` declares `out(d0 if eq)`")
}

/// CHK <ea>, Dn traps when Dn < 0 or Dn > the bound and otherwise falls
/// through. It writes no register (an auto-increment source still advances its
/// base); N is defined only on the trap and Z, V, C are "Undefined", so to every
/// flag check the flags after it are gone. It does not end a routine.
#[test]
fn chk_reads_its_operands_and_leaves_the_flags_undefined() {
    // The bound register is read, never written.
    assert_eq!(diags("clobbers()", "    chk.w d2, d1\n    rts"), []);
    let d = diags("clobbers()", "    chk.w (a1)+, d1\n    rts");
    assert!(has(&d, "[proc.clobber-undeclared] `P` writes `a1`"), "{d:?}");
    assert!(!has(&d, "[proc.clobber-undeclared] `P` writes `d1`"), "{d:?}");
    // Not a terminator.
    let d = diags("clobbers()", "    chk.w d2, d1");
    assert!(has(&d, "[proc.undeclared-fallthrough] `P` can reach its closing `}`"), "{d:?}");
    // The flag-result walk: a callee's carry (or zero) is gone after `chk`, so
    // the branch after it does not read it and the result is unused (fires).
    // Control: `lea` leaves the flags, the branch reads the callee's.
    for (flag, branch) in [("carry", "bcs.s"), ("zero", "beq.s")] {
        for (mid, fires) in [("chk.w d2, d1", true), ("lea (a0), a1", false)] {
            let src = format!("module m\nproc P () clobbers(a1) {{\n    jbsr Queue\n    {mid}\n    {branch} .x\n.x:\n    rts\n}}\n");
            let f = flag_unused(&src, flag);
            assert_eq!(!f.is_empty(), fires, "{flag} across `{mid}`: {f:?}");
        }
    }
    // The conditional-out tracker: the flags are unknown after `chk`.
    assert!(!out_cond_survives("chk.w d2, d1"), "flags across `chk`");
    assert!(out_cond_survives("lea (a0), a1"), "control: `lea` keeps them");
    // `preserves(sr.ccr)` is refused. Control: `lea` keeps it.
    let d = diags("preserves(sr.ccr)", "    chk.w d2, d1\n    rts");
    assert!(errors(&d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)`")), "{d:?}");
    assert_eq!(diags("clobbers(a1) preserves(sr.ccr)", "    lea (a0), a1\n    rts"), [], "control");
}

/// TRAPV traps when V is set (it READS V, which no check tracks) and otherwise
/// falls through with the condition codes untouched ("Condition Codes: Not
/// affected."). It writes no register and does not end a routine.
#[test]
fn trapv_falls_through_with_the_flags_it_found() {
    assert_eq!(diags("clobbers()", "    trapv\n    rts"), []);
    let d = diags("clobbers()", "    trapv");
    assert!(has(&d, "[proc.undeclared-fallthrough] `P` can reach its closing `}`"), "{d:?}");
    // The flag-result walk: a callee's carry (or zero) read after `trapv` is
    // still the callee's. Control: `move.l a0, d0` writes both flags.
    for (flag, branch) in [("carry", "bcs.s"), ("zero", "beq.s")] {
        for (mid, fires) in [("trapv", false), ("move.l a0, d0", true)] {
            let src = format!("module m\nproc P () clobbers(d0) {{\n    jbsr Queue\n    {mid}\n    {branch} .x\n.x:\n    rts\n}}\n");
            let f = flag_unused(&src, flag);
            assert_eq!(!f.is_empty(), fires, "{flag} across `{mid}`: {f:?}");
        }
    }
    // The conditional-out tracker still knows the flags after `trapv` (fires).
    assert!(out_cond_survives("trapv"), "flags across `trapv`");
    assert!(!out_cond_survives("move.l a0, d2"), "control: `move.l a0, d2` writes the flags");
    // `preserves(sr.ccr)` holds across it. Control: a flag write does not.
    assert_eq!(diags("preserves(sr.ccr)", "    trapv\n    rts"), []);
    let d = diags("clobbers(d0) preserves(sr.ccr)", "    trapv\n    moveq #0, d0\n    rts");
    assert!(errors(&d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(sr.ccr)`")), "control: {d:?}");
    // The constant-branch net: `moveq #0` sets Z, `trapv` keeps it on the path
    // that reaches the `beq`, so the branch is always taken. Control: `chk`.
    let src = |mid: &str| format!("module m\nproc P () clobbers(d0) {{\n    moveq #0, d0\n    {mid}\n    beq.s .x\n.x:\n    rts\n}}\n");
    let f = branch_const(&src("trapv"));
    assert!(f.len() == 1 && f[0].cc == "eq" && f[0].always_taken, "{f:?}");
    assert_eq!(branch_const(&src("chk.w d2, d1")), [], "control: `chk` leaves Z undefined");
}

/// LINK's displacement is a signed 16-bit extension word: `#$FFF8` IS -8 (the
/// same bytes, `4E56 FFF8`), so the stack check reads it as -8 and reports an
/// open frame exactly as it does for `#-8`.
#[test]
fn a_link_displacement_is_its_signed_word() {
    let open = |imm: &str| diags("clobbers(a6)", &format!("    link a6, #{imm}\n    rts"));
    let minus8 = open("-8");
    assert_eq!(errors(&minus8).len(), 1, "{minus8:?}");
    assert!(errors(&minus8)[0].starts_with("[stack.unbalanced] in `P`: this path returns with 12 bytes"), "{minus8:?}");
    assert_eq!(open("$FFF8"), minus8, "`#$FFF8` is the same instruction as `#-8`");
    assert_eq!(diags("clobbers(a6)", "    link a6, #$FFF8\n    unlk a6\n    rts"), []);
    // The boundary: `$8000` is -32768 (4 + 32768 bytes open), the same as `#-$8000`.
    let low = open("$8000");
    assert!(errors(&low).first().is_some_and(|m| m.starts_with("[stack.unbalanced] in `P`: this path returns with 32772 bytes")), "{low:?}");
    assert_eq!(open("-$8000"), low);
    assert_eq!(diags("clobbers(a6)", "    link a6, #$8000\n    unlk a6\n    rts"), []);
    // `$7FFF` is +32767: sp moves UP, into the caller's frame. The model does not
    // represent that and gives up on the path, as it does for `#8`.
    assert_eq!(open("$7FFF"), []);
    assert_eq!(open("8"), []);
    // Outside the word the encoder refuses the line, and the stack check adds nothing.
    let d = open("-$8001");
    assert!(errors(&d).iter().any(|m| m.contains("out of range")), "{d:?}");
    assert!(!has(&d, "[stack.unbalanced]"), "{d:?}");
}

/// `unlk aN` loads sp from aN: it closes the frame only if aN still holds the sp
/// `link` put there. A write of aN inside the frame breaks that, so the `unlk`
/// leaves sp at a value the model never computed: the path's depth is unknown
/// (no `[stack.unbalanced]` verdict either way) and `preserves(a6)` is
/// unverifiable, never satisfied.
#[test]
fn writing_the_frame_register_inside_its_frame_unpairs_the_unlk() {
    let hdr = "clobbers(d0, a0, a5) preserves(a6)";
    let framed = |mid: &str| diags(hdr, &format!("    link a6, #-8\n    {mid}\n    unlk a6\n    rts"));
    for mid in ["movea.l a0, a6", "lea 4(a6), a6", "exg a0, a6", "addq.l #4, a6", "suba.l a0, a6", "movem.l (a0), d0/a6"] {
        let d = framed(mid);
        assert!(!has(&d, "[stack.unbalanced]"), "`{mid}`: {d:?}");
        assert!(
            errors(&d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(a6)`")),
            "`{mid}`: preserves(a6) must not pass: {d:?}"
        );
    }
    // The frame is still open at the return (no `unlk`): the depth is known and reported.
    let d = diags(hdr, "    link a6, #-8\n    movea.l a0, a6\n    rts");
    assert!(has(&d, "[stack.unbalanced] in `P`: this path returns with 12 bytes"), "{d:?}");
    // Controls: reading a6, writing through it, writing another address register.
    for mid in ["move.l -4(a6), d0", "move.l d0, -4(a6)", "movea.l a0, a5"] {
        assert_eq!(framed(mid), [], "`{mid}`");
    }
    // A nested frame on the same register is modelled exactly: the inner `unlk`
    // restores a6 to the outer frame's sp.
    assert_eq!(diags(hdr, "    link a6, #-8\n    link a6, #-4\n    unlk a6\n    unlk a6\n    rts"), []);
    let d = diags(hdr, "    link a6, #-8\n    link a6, #-4\n    unlk a6\n    rts");
    assert!(has(&d, "[stack.unbalanced] in `P`: this path returns with 12 bytes"), "{d:?}");
}


/// MOVE to CCR is a word operation whatever suffix is written: `move.b` and
/// `move.w` give the same bytes (`44D8`) and the CPU reads a WORD, so
/// `(a0)+` / `-(a0)` move `a0` by 2. The delta model must charge 2, not the
/// written `.b`'s 1: the correct restore verifies, the off-by-one one does not.
#[test]
fn a_byte_sized_move_to_ccr_moves_its_address_register_by_a_word() {
    let hdr = "clobbers() preserves(a0)";
    let unverifiable = |d: &[(bool, String)]| {
        errors(d).iter().any(|m| m.starts_with("[proc.preserves-unverifiable] `P` declares `preserves(a0)`"))
    };
    for (src, fix) in [("(a0)+", "lea -2(a0), a0"), ("-(a0)", "lea 2(a0), a0")] {
        for sz in ["b", "w"] {
            let ok = diags(hdr, &format!("    move.{sz} {src}, ccr\n    {fix}\n    rts"));
            assert_eq!(ok, [], "`move.{sz} {src}, ccr` then `{fix}` restores a0");
        }
        let wrong = if src == "(a0)+" { "lea -1(a0), a0" } else { "lea 1(a0), a0" };
        let d = diags(hdr, &format!("    move.b {src}, ccr\n    {wrong}\n    rts"));
        assert!(unverifiable(&d), "`move.b {src}, ccr` then `{wrong}` leaves a0 off by one: {d:?}");
    }
    // Control: an ordinary byte move from `(a0)+` advances a0 by 1.
    assert_eq!(diags("clobbers(d0) preserves(a0)", "    move.b (a0)+, d0\n    lea -1(a0), a0\n    rts"), []);
}

/// The other consumer of the operand width on the same line: the field-overrun
/// lint. `move.b flags(a0), ccr` reads the WORD at `flags`, so the CCR gets the
/// NEXT field's byte; against a 1-byte field that is an overrun, as `move.w` is.
#[test]
fn a_byte_sized_move_to_ccr_reads_a_word_of_its_field() {
    let probe = |line: &str| {
        module_diags(&format!(
            "module m\nstruct S (size: 4) {{\n    flags: u8,\n    next: u8,\n    w: u16,\n}}\nproc P (a0: *S) clobbers(d0) {{\n    {line}\n    rts\n}}\n"
        ))
    };
    for line in ["move.b flags(a0), ccr", "move.w flags(a0), ccr"] {
        let d = probe(line);
        assert!(has(&d, "[operand.field-overrun]"), "`{line}`: {d:?}");
    }
    // Controls: a word field, and a byte move to a data register.
    for line in ["move.b w(a0), ccr", "move.w w(a0), ccr", "move.b flags(a0), d0"] {
        assert_eq!(probe(line), [], "`{line}`");
    }
}
