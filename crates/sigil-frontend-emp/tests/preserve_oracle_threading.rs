//! The register-preserve oracle inputs thread the REAL `@noreturn` set and
//! mask-preservers map, not empty stand-ins. Both `preserve_oracle_inputs` and
//! `verified_preserves_regs` run `check_preserves`, whose mask-claim tail credit
//! (`sr_mask_preservers_credit`) consults the map; a miss is an ERROR that zeroes
//! the register credit. The rot the threading guards: a proc that
//! register-preserves AND claims the mask through an unconditional external tail
//! loses its register credit the day the map silently reverts to empty.
//!
//! Two witnesses, because the threading's force differs by shape:
//! - REACHABLE terminal tail — inert: the tail poisons every register under
//!   `ClobberAll`, so the base credit is empty either way, and only the CORPUS
//!   oracle input (which the closure round later upgrades) moves.
//! - UNREACHABLE terminal tail (dead code) — load-bearing on the base path too:
//!   `terminal_external_tail` classifies on the LAST instruction regardless of
//!   reachability, while `verify_preserved` is reachability-based, so the live
//!   return path round-trips a register the empty-map mask refusal would discard.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::lower::{preserve_oracle_inputs, verified_preserves_regs};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use std::collections::{BTreeMap, BTreeSet};

/// Parse `src`, evaluate its `keeper` proc, and return the decl + CodeBuf.
fn keeper_of(src: &str) -> (sigil_frontend_emp::ast::File, sigil_frontend_emp::value::CodeBuf) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let p = file
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) if p.name == "keeper" => Some(p),
            _ => None,
        })
        .expect("keeper proc");
    let (buf, _d, _n) = eval_proc_body(
        &file,
        &p.name,
        &p.params,
        &p.body,
        p.span,
        0,
        Cpu::M68000,
        &[],
        &sigil_frontend_emp::contract::InterfaceEnv::empty(),
    );
    let buf = buf.expect("codebuf");
    (file, buf)
}

/// The `keeper` decl out of an evaluated file.
fn decl(file: &sigil_frontend_emp::ast::File) -> &sigil_frontend_emp::ast::ProcDecl {
    file.items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) if p.name == "keeper" => Some(p),
            _ => None,
        })
        .unwrap()
}

/// `keeper` preserves `a0` + `sr.mask` and leaves through a REACHABLE terminal
/// `jbra Sibling`.
const REACHABLE_TAIL: &str = "module m\n\
                              extern proc Sibling() preserves(sr.mask)\n\
                              proc keeper() preserves(a0, sr.mask) {\n\
                                  jbra Sibling\n\
                              }\n";

/// Same contract, but the terminal `jbra Sibling` is DEAD — the `rts` above it is
/// the only reachable exit.
const DEAD_TAIL: &str = "module m\n\
                         extern proc Sibling() preserves(sr.mask)\n\
                         proc keeper() preserves(a0, sr.mask) {\n\
                             rts\n\
                             jbra Sibling\n\
                         }\n";

/// A one-entry mask-preservers map crediting a plain-name tail into `owner`.
fn map_with(owner: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut m = BTreeMap::new();
    m.insert(owner.to_string(), BTreeSet::new());
    m
}

#[test]
fn oracle_inputs_need_the_real_mask_preservers_map() {
    let (file, buf) = keeper_of(REACHABLE_TAIL);
    let p = decl(&file);
    let noreturn = BTreeSet::new();

    // WITH the real map: the mask-tail to `Sibling` is credited, `check_preserves`
    // emits no error, and the oracle input carries `a0` for the closure round.
    let (check_real, names_real) = preserve_oracle_inputs(p, &buf, &noreturn, &map_with("Sibling"));
    assert!(
        !check_real.is_empty() && names_real.contains("a0"),
        "real map must keep the a0 oracle input: check={check_real:?} names={names_real:?}"
    );

    // WITH an empty map (the rot): the mask-tail refuses with
    // `[proc.preserves-sr-unbalanced]`, which zeroes the oracle input — the closure
    // can no longer re-credit `a0` through the tail.
    let (check_empty, names_empty) =
        preserve_oracle_inputs(p, &buf, &noreturn, &BTreeMap::new());
    assert!(
        check_empty.is_empty() && names_empty.is_empty(),
        "empty map must collapse the oracle input (the rot): check={check_empty:?} names={names_empty:?}"
    );
}

#[test]
fn base_credit_needs_the_real_map_when_the_terminal_tail_is_dead() {
    // The load-bearing shape for the BASE path. `terminal_external_tail` sees the
    // trailing `jbra Sibling` and demands a mask credit for it; `verify_preserved`
    // never walks it (unreachable past the `rts`), so a0 round-trips on the live
    // path. Real map → the mask claim is discharged and a0 is credited; empty map →
    // the mask refusal errors first and the credit is discarded. This is the
    // polarity pair that proves the threading is not merely cosmetic.
    let (file, buf) = keeper_of(DEAD_TAIL);
    let p = decl(&file);
    let noreturn = BTreeSet::new();

    let real = verified_preserves_regs(p, &buf, &noreturn, &map_with("Sibling"));
    assert!(
        real.contains("a0"),
        "dead terminal tail + real map must credit a0, got {real:?}"
    );

    let empty = verified_preserves_regs(p, &buf, &noreturn, &BTreeMap::new());
    assert!(
        empty.is_empty(),
        "dead terminal tail + empty map must discard the credit (the rot), got {empty:?}"
    );
}

#[test]
fn base_credit_is_tail_poisoned_when_the_terminal_tail_is_reachable() {
    // The INERT shape, asserted for its own reason rather than as a proxy: with a
    // REACHABLE terminal tail the base credit is empty under both maps — but for
    // DIFFERENT causes, so this is a scope statement, not a witness that the maps
    // agree. Real map: the mask claim is discharged, then `ClobberAll` poisons a0
    // through the live tail. Empty map: the mask refusal errors before the register
    // walk is ever reached. The reachable shape is why the frozen corpus measures
    // byte-identical; `base_credit_needs_the_real_map_when_the_terminal_tail_is_dead`
    // is why the threading still matters.
    let (file, buf) = keeper_of(REACHABLE_TAIL);
    let p = decl(&file);
    let noreturn = BTreeSet::new();

    // Non-vacuity: the real map genuinely DISCHARGES the mask claim here (no
    // error) — proven by the oracle-inputs path in the test above being non-empty
    // for this same shape. So the emptiness below is the ClobberAll tail poison,
    // not a silent mask refusal.
    let (check_real, _) = preserve_oracle_inputs(p, &buf, &noreturn, &map_with("Sibling"));
    assert!(!check_real.is_empty(), "guard: the real map must discharge the mask claim here");

    assert!(
        verified_preserves_regs(p, &buf, &noreturn, &map_with("Sibling")).is_empty(),
        "reachable tail: ClobberAll poisons a0 even with the mask claim discharged"
    );
    assert!(
        verified_preserves_regs(p, &buf, &noreturn, &BTreeMap::new()).is_empty(),
        "reachable tail: the empty map errors on the mask claim before the register walk"
    );
}
