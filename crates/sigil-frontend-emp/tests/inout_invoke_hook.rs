//! INOUT-PROOF-INVOKE-HOOK (Stage 0 probe). `invoke Iface.hook` with a BOUND hook
//! lowers to `jsr (Sym).l`, an `AbsSym` operand. The inout verifier resolves a
//! call's callee through `out_verify::direct_target`, which reads only a bare
//! `Sym`, so a bound hook site reaches `inout_call_disposition` with no target and
//! is charged as an unknown callee, even when the bound proc is a known key of
//! the effective-clobber map. These tests pin that gap, one direction each, and a
//! bare-`jsr` control that proves the callee maps are correct.
//!
//! The gap test, `invoke_of_preserving_hook_is_charged_unknown`, asserts the
//! CURRENT (gap) behaviour. A fix that lets the verifier resolve an `AbsSym`
//! target turns it red; the fix then flips that assertion to "verifies", and the
//! include-direction test must stay green unchanged.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::contract::{InterfaceEnv, ResolvedInterface, ResolvedMember};
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::out_verify::{verify_inout, InoutCallees, OutClaim, OutWidth, OutWidths};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{CodeItem, Reg};
use sigil_ir::backend::Cpu;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// An env with one interface `Game` whose hook `tick` is bound to `Bound`.
fn env_with_bound_hook() -> InterfaceEnv {
    let members = HashMap::from([(
        "tick".to_string(),
        ResolvedMember::Hook(Some("Bound".to_string())),
    )]);
    InterfaceEnv {
        interfaces: HashMap::from([("Game".to_string(), ResolvedInterface { members })]),
    }
}

/// Eval proc `name` in `src` under `env`, returning its CodeItems.
fn eval_proc(src: &str, name: &str, env: &InterfaceEnv) -> Vec<CodeItem> {
    let (file, diags) = parse_str(src);
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {diags:?}");
    let p = file
        .items
        .iter()
        .find_map(|it| match it {
            Item::Proc(p) if p.name == name => Some(p),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no proc {name}"));
    let (buf, d, _) = eval_proc_body(
        &file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[], env,
    );
    assert!(d.iter().all(|x| x.level != sigil_span::Level::Error), "eval: {d:?}");
    buf.expect("proc body evaluates").items
}

/// Verify `inout(d5: u16)` on `items` with `Bound` as the only known callee,
/// clobbering `bound_clobbers`. Returns the exit status (`None` = verified).
fn d5_status(items: &[CodeItem], bound_clobbers: &[&str]) -> Option<String> {
    let own = BTreeMap::from([("d5".to_string(), OutClaim::exact(OutWidth::W))]);
    let no_widths = BTreeMap::new();
    let empty: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let cond: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let clob: BTreeMap<String, BTreeSet<String>> = BTreeMap::from([(
        "Bound".to_string(),
        bound_clobbers.iter().map(|s| s.to_string()).collect(),
    )]);
    let mut st = verify_inout(
        items,
        &[Reg::D5],
        OutWidths { own: &own, callees: &no_widths },
        InoutCallees { uncond_out: &empty, inout: &empty, cond_out: &cond, effective_clobbers: &clob },
        None,
        &BTreeSet::new(),
    );
    st.remove(&Reg::D5).expect("a status for d5")
}

const INVOKER: &str =
    "module m\n proc P (d5: u16) clobbers(d0-d4/a1-a2) inout(d5: u16) {\n invoke Game.tick\n rts\n }\n";
const BARE_JSR: &str =
    "module m\n proc P (d5: u16) clobbers(d0-d4/a1-a2) inout(d5: u16) {\n jsr Bound\n rts\n }\n";

/// The invoke really lowers to a call naming `Bound` (so the probe is not
/// vacuous: an unbound hook would emit nothing and trivially verify).
#[test]
fn bound_invoke_emits_a_call_item() {
    let items = eval_proc(INVOKER, "P", &env_with_bound_hook());
    let calls = items.iter().filter(|it| format!("{it:?}").contains("Bound")).count();
    assert_eq!(calls, 1, "exactly one item names the bound proc: {items:?}");
}

/// CONTROL: a bare `jsr Bound` against the same maps verifies (Bound clobbers
/// d0-d4/a1-a2, not d5). Proves the maps, so the next test's firing is the operand shape.
#[test]
fn bare_jsr_of_preserving_callee_verifies() {
    let items = eval_proc(BARE_JSR, "P", &InterfaceEnv::empty());
    assert_eq!(d5_status(&items, &["d0", "d1", "d2", "d3", "d4", "a1", "a2"]), None);
}

/// THE GAP (exclude direction): the bound hook's proc does not clobber d5, and
/// the verifier still fires, with the unknown-callee reason.
#[test]
fn invoke_of_preserving_hook_is_charged_unknown() {
    let items = eval_proc(INVOKER, "P", &env_with_bound_hook());
    let st = d5_status(&items, &["d0", "d1", "d2", "d3", "d4", "a1", "a2"]);
    let reason = st.expect("GAP PINNED: the invoke of a d5-preserving hook currently fires");
    assert!(reason.contains("indirect or unknown callee"), "reason: {reason}");
}

/// Include direction: a bound proc that clobbers d5 must fire, before and after
/// any fix. Only the reason may change (unknown callee now, callee clobber after).
#[test]
fn invoke_of_clobbering_hook_fires() {
    let items = eval_proc(INVOKER, "P", &env_with_bound_hook());
    let st = d5_status(&items, &["d0", "d5"]);
    assert!(st.is_some(), "a d5-clobbering hook must break inout(d5)");
}
