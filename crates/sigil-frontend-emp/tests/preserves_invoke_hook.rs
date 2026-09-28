//! PRESERVES-CALL-TARGET-ABSSYM. `invoke Iface.hook` with a BOUND hook lowers to
//! `jsr (Sym).l`, an `AbsSym` operand. The two `preserves.rs` consumers of "which
//! proc does this call name" (the verified-`preserves` call transfer under the
//! closure oracle, and the `[proc.dead-save]` walk) read that target as the bound
//! proc through `flag_check::direct_proc_target`, so a bound hook site is judged
//! by the bound proc's `effective` clobbers exactly as a bare `jsr` of that proc
//! is. A bound name the `effective` map does not hold stays an unknown callee,
//! and a `$`-mangled local label is never looked up as a proc.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::closure::RegEffect;
use sigil_frontend_emp::contract::{InterfaceEnv, ResolvedInterface, ResolvedMember};
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::preserves::{find_dead_saves, verify_preserved, CallPolicy, PreserveStatus};
use sigil_frontend_emp::value::{CodeItem, CodeOperand, Reg};
use sigil_ir::backend::Cpu;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// An env with one interface `Game` whose hook `tick` is bound to `bound`.
fn env_binding(bound: &str) -> InterfaceEnv {
    let members = HashMap::from([(
        "tick".to_string(),
        ResolvedMember::Hook(Some(bound.to_string())),
    )]);
    InterfaceEnv {
        interfaces: HashMap::from([("Game".to_string(), ResolvedInterface { members })]),
    }
}

/// Eval the first proc in `src` under `env`, returning its CodeItems.
fn eval_items(src: &str, env: &InterfaceEnv) -> Vec<CodeItem> {
    let (file, diags) = parse_str(src);
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {diags:?}");
    let p = file
        .items
        .iter()
        .find_map(|it| match it {
            Item::Proc(p) => Some(p),
            _ => None,
        })
        .expect("a proc");
    let (buf, d, _) =
        eval_proc_body(&file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[], env);
    assert!(d.iter().all(|x| x.level != sigil_span::Level::Error), "eval: {d:?}");
    buf.expect("proc body evaluates").items
}

/// An `effective` map where each callee clobbers exactly the listed registers.
fn eff(entries: &[(&str, &[&str])]) -> BTreeMap<String, RegEffect> {
    entries
        .iter()
        .map(|(c, clob)| {
            (
                c.to_string(),
                RegEffect { top: false, regs: clob.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>() },
            )
        })
        .collect()
}

/// The verified-`preserves` verdict for `reg` under the closure oracle.
fn preserve_status(items: &[CodeItem], reg: Reg, effective: &BTreeMap<String, RegEffect>) -> PreserveStatus {
    let mut st = verify_preserved(items, &[reg], CallPolicy::Oracle(effective), None, &BTreeSet::new());
    st.remove(&reg).expect("a status for the checked register")
}

/// The dead-save firings as (register, bracketed callees).
fn dead_saves(items: &[CodeItem], effective: &BTreeMap<String, RegEffect>) -> Vec<(Reg, Vec<String>)> {
    let mut out: Vec<(Reg, Vec<String>)> =
        find_dead_saves("P", items, effective).into_iter().map(|d| (d.reg, d.callees)).collect();
    out.sort();
    out
}

/// The one call instruction's operands (the probes below assert on its shape).
fn call_ops(items: &[CodeItem]) -> Vec<CodeOperand> {
    let calls: Vec<&Vec<CodeOperand>> = items
        .iter()
        .filter_map(|it| match it {
            CodeItem::Instr { mnemonic, ops, .. } if matches!(mnemonic.as_str(), "jsr" | "jbsr" | "bsr") => Some(ops),
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 1, "exactly one call item: {items:?}");
    calls[0].clone()
}

/// `Bound` clobbers d0-d4/a1-a2 and so PRESERVES d5.
const BOUND_PRESERVES_D5: &[&str] = &["d0", "d1", "d2", "d3", "d4", "a1", "a2"];

// ---- verified `preserves` (the call transfer under `CallPolicy::Oracle`) ----

/// A proc that calls the hook with no save of d5: `preserves(d5)` holds iff the
/// call preserves d5.
const PRES_INVOKE: &str = "module m\n proc P () clobbers(d0-d4/a1-a2) {\n invoke Game.tick\n rts\n }\n";
const PRES_BARE: &str = "module m\n proc P () clobbers(d0-d4/a1-a2) {\n jsr Bound\n rts\n }\n";

/// The invoke lowers to a SOLE `AbsSym` operand naming the bound proc, so the
/// tests below exercise that operand kind and not an empty hook.
#[test]
fn bound_invoke_lowers_to_a_sole_abssym() {
    let ops = call_ops(&eval_items(PRES_INVOKE, &env_binding("Bound")));
    assert!(
        matches!(ops.as_slice(), [CodeOperand::AbsSym { target, .. }] if target == "Bound"),
        "invoke lowers to jsr (Bound).l: {ops:?}"
    );
}

/// CONTROL: a bare `jsr Bound` against the same map verifies `preserves(d5)`.
#[test]
fn bare_jsr_of_preserving_callee_verifies_preserves() {
    let items = eval_items(PRES_BARE, &InterfaceEnv::empty());
    let e = eff(&[("Bound", BOUND_PRESERVES_D5)]);
    assert_eq!(preserve_status(&items, Reg::D5, &e), PreserveStatus::Verified);
}

/// The bound proc preserves d5, so the invoke preserves it: the same verdict as
/// the bare-`jsr` control.
#[test]
fn invoke_of_preserving_hook_verifies_preserves() {
    let items = eval_items(PRES_INVOKE, &env_binding("Bound"));
    let e = eff(&[("Bound", BOUND_PRESERVES_D5)]);
    assert_eq!(preserve_status(&items, Reg::D5, &e), PreserveStatus::Verified);
}

/// A bound proc that clobbers d5 refuses `preserves(d5)`.
#[test]
fn invoke_of_clobbering_hook_refuses_preserves() {
    let items = eval_items(PRES_INVOKE, &env_binding("Bound"));
    let e = eff(&[("Bound", &["d0", "d5"])]);
    assert_eq!(preserve_status(&items, Reg::D5, &e), PreserveStatus::NotPreserved);
}

/// A bound proc absent from `effective` is an unknown callee: naming it is only a
/// lookup key, never a proof by itself.
#[test]
fn invoke_of_uncontracted_bound_proc_refuses_preserves() {
    let items = eval_items(PRES_INVOKE, &env_binding("Elsewhere"));
    let e = eff(&[("Bound", BOUND_PRESERVES_D5)]);
    assert_eq!(preserve_status(&items, Reg::D5, &e), PreserveStatus::NotPreserved);
}

// ---- `[proc.dead-save]` ----

const DS_INVOKE: &str = "module m\n proc P () clobbers(d0-d4/a1-a2) {\n move.l d5, -(sp)\n invoke Game.tick\n move.l (sp)+, d5\n rts\n }\n";
const DS_BARE: &str = "module m\n proc P () clobbers(d0-d4/a1-a2) {\n move.l d5, -(sp)\n jsr Bound\n move.l (sp)+, d5\n rts\n }\n";

/// CONTROL: a save of d5 around a bare `jsr` of a d5-preserving proc is dead.
#[test]
fn save_around_bare_jsr_of_preserving_callee_is_dead() {
    let items = eval_items(DS_BARE, &InterfaceEnv::empty());
    let e = eff(&[("Bound", BOUND_PRESERVES_D5)]);
    assert_eq!(dead_saves(&items, &e), vec![(Reg::D5, vec!["Bound".to_string()])]);
}

/// The same save around the invoke is dead too, credited to the bound proc.
#[test]
fn save_around_invoke_of_preserving_hook_is_dead() {
    let items = eval_items(DS_INVOKE, &env_binding("Bound"));
    let e = eff(&[("Bound", BOUND_PRESERVES_D5)]);
    assert_eq!(dead_saves(&items, &e), vec![(Reg::D5, vec!["Bound".to_string()])]);
}

/// A bound proc that clobbers d5 needs the save: no firing.
#[test]
fn save_around_invoke_of_clobbering_hook_is_needed() {
    let items = eval_items(DS_INVOKE, &env_binding("Bound"));
    let e = eff(&[("Bound", &["d0", "d5"])]);
    assert_eq!(dead_saves(&items, &e), vec![]);
}

/// A bound proc absent from `effective` needs the save: no firing.
#[test]
fn save_around_invoke_of_uncontracted_bound_proc_is_needed() {
    let items = eval_items(DS_INVOKE, &env_binding("Elsewhere"));
    let e = eff(&[("Bound", BOUND_PRESERVES_D5)]);
    assert_eq!(dead_saves(&items, &e), vec![]);
}

// ---- the `$` axis: a local helper is never a proc ----

/// `jbsr .helper` runs code INSIDE this proc, not a proc's entry, so no proc
/// contract describes it: it is charged as an unknown callee even if the map were
/// to hold its mangled symbol as a key. The map below does hold it (a key the real
/// pipeline cannot produce, since a `$`-mangled spelling is unspellable from
/// source), so a reader that looked the mangled name up would credit the helper
/// with preserving d5.
const LOCAL: &str = "module m\n proc P () clobbers(d0-d4/a1-a2) {\n move.l d5, -(sp)\n jbsr .helper\n move.l (sp)+, d5\n rts\n .helper:\n moveq #0, d5\n rts\n }\n";

fn local_items_and_symbol() -> (Vec<CodeItem>, String) {
    let items = eval_items(LOCAL, &InterfaceEnv::empty());
    let sym = match call_ops(&items).as_slice() {
        [CodeOperand::Sym(s)] => s.clone(),
        other => panic!("jbsr .helper lowers to a sole Sym: {other:?}"),
    };
    assert!(sym.contains('$'), "the local helper's symbol is mangled: {sym}");
    (items, sym)
}

/// The same helper called with no save: the call is the only thing between entry
/// and the proc's `rts` (the helper's block is reachable only through the call),
/// so the verdict is exactly how the call is charged. The helper writes d5.
const LOCAL_NO_SAVE: &str = "module m\n proc P () clobbers(d0-d4/a1-a2) {\n jbsr .helper\n rts\n .helper:\n moveq #0, d5\n rts\n }\n";

#[test]
fn local_helper_is_not_credited_by_preserves() {
    let (_, sym) = local_items_and_symbol();
    let items = eval_items(LOCAL_NO_SAVE, &InterfaceEnv::empty());
    assert!(
        matches!(call_ops(&items).as_slice(), [CodeOperand::Sym(s)] if *s == sym),
        "the no-save probe calls the same mangled helper"
    );
    let e = eff(&[(sym.as_str(), &["d0"])]);
    assert_eq!(
        preserve_status(&items, Reg::D5, &e),
        PreserveStatus::NotPreserved,
        "a local helper that writes d5 is never credited with preserving it"
    );
}

#[test]
fn local_helper_save_is_never_dead() {
    let (items, sym) = local_items_and_symbol();
    let e = eff(&[(sym.as_str(), &["d0"])]);
    assert_eq!(dead_saves(&items, &e), vec![], "a save around a local helper is needed");
}
