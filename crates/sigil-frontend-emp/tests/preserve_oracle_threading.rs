//! Item-2 pin: the register-preserve oracle inputs thread the REAL mask-preservers
//! map, not an empty stand-in. `preserve_oracle_inputs` runs `check_preserves`,
//! whose mask-claim tail credit ([`sr_mask_preservers_credit`]) consults the map;
//! a miss is an ERROR that zeroes the oracle input (`(∅, ∅)`). The rot the
//! threading guards: a proc that register-preserves AND claims the mask through an
//! unconditional external tail loses its register credit forever the day the map
//! silently reverts to empty — even though the corpus closure could re-prove the
//! register through the tail-callee.
//!
//! The witness is `keeper`: it preserves `a0` (never written → optimistically
//! provable, deferred to the closure) AND `sr.mask`, leaving through `jbra Sibling`
//! where `Sibling` preserves the mask. With the mask-preservers map naming
//! `Sibling`, the tail is credited and the oracle input carries `a0`; with an empty
//! map the mask-tail refuses and the input collapses to empty.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::lower::{preserve_oracle_inputs, verified_preserves_regs};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use std::collections::{BTreeMap, BTreeSet};

/// The `keeper` proc + its `Sibling` tail target, evaluated to a CodeBuf.
fn keeper_buf() -> (sigil_frontend_emp::ast::File, sigil_frontend_emp::value::CodeBuf) {
    let src = "module m\n\
               extern proc Sibling() preserves(sr.mask)\n\
               proc keeper() preserves(a0, sr.mask) {\n\
                   jbra Sibling\n\
               }\n";
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

/// A one-entry mask-preservers map crediting a plain-name tail into `owner`.
fn map_with(owner: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut m = BTreeMap::new();
    m.insert(owner.to_string(), BTreeSet::new());
    m
}

#[test]
fn oracle_inputs_need_the_real_mask_preservers_map() {
    let (file, buf) = keeper_buf();
    let p = file
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) if p.name == "keeper" => Some(p),
            _ => None,
        })
        .unwrap();
    let noreturn = BTreeSet::new();
    let real = map_with("Sibling");
    let empty: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    // WITH the real map: the mask-tail to `Sibling` is credited, `check_preserves`
    // emits no error, and the oracle input carries `a0` for the closure round.
    let (check_real, names_real) = preserve_oracle_inputs(p, &buf, &noreturn, &real);
    assert!(
        !check_real.is_empty() && names_real.contains("a0"),
        "real map must keep the a0 oracle input: check={check_real:?} names={names_real:?}"
    );

    // WITH an empty map (the rot): the mask-tail refuses with
    // `[proc.preserves-sr-unbalanced]`, which zeroes the oracle input — the closure
    // can no longer re-credit `a0` through the tail. This is the polarity the
    // threading prevents.
    let (check_empty, names_empty) = preserve_oracle_inputs(p, &buf, &noreturn, &empty);
    assert!(
        check_empty.is_empty() && names_empty.is_empty(),
        "empty map must collapse the oracle input (the rot): check={check_empty:?} names={names_empty:?}"
    );
}

#[test]
fn base_verified_preserves_is_tail_poisoned_either_way() {
    // Corroborating witness: the BASE `verified_preserves_regs` (ClobberAll) is
    // structurally inert to the mask map for this shape — the terminal external
    // tail poisons every register under ClobberAll, so a0 is uncredited with the
    // real map too. This is WHY the rot lives in `preserve_oracle_inputs` (whose
    // result the closure upgrades), not the base path: the base never credits a
    // tailing proc's registers regardless of the mask verdict.
    let (file, buf) = keeper_buf();
    let p = file
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) if p.name == "keeper" => Some(p),
            _ => None,
        })
        .unwrap();
    let noreturn = BTreeSet::new();
    let real = map_with("Sibling");
    let empty: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    assert!(
        verified_preserves_regs(p, &buf, &noreturn, &real).is_empty(),
        "base ClobberAll credit is tail-poisoned even with the real map"
    );
    assert!(
        verified_preserves_regs(p, &buf, &noreturn, &empty).is_empty(),
        "base ClobberAll credit is empty with the empty map too"
    );
}
