//! INOUT-PROOF-INVOKE-HOOK. `invoke Iface.hook` with a BOUND hook lowers to
//! `jsr (Sym).l`, an `AbsSym` operand. Every caller-side reader that asks "which
//! proc does this call name" reads that `AbsSym` target as the bound proc
//! (`flag_check::direct_proc_target`), so a bound hook site is judged by the
//! bound proc's contract exactly as a bare `jsr` of that proc is. A bound name the
//! caller's maps do not hold stays an unknown callee.
//!
//! Sections: the inout verifier (`out_verify`), the D1b/D1c call gates
//! (`calls`), and the domain-newtype slot check (`type_slice`).

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::calls::{check_input_undefined, check_live_clobbered};
use sigil_frontend_emp::closure::RegEffect;
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

/// Exclude direction: the bound hook's proc does not clobber d5, so the invoke
/// preserves it and the inout proof verifies, the same verdict as the bare-`jsr`
/// control above.
#[test]
fn invoke_of_preserving_hook_verifies() {
    let items = eval_proc(INVOKER, "P", &env_with_bound_hook());
    assert_eq!(d5_status(&items, &["d0", "d1", "d2", "d3", "d4", "a1", "a2"]), None);
}

/// A bound proc with no entry in the effective-clobber map stays an unknown
/// callee: naming the target is only a lookup key, never a proof by itself.
#[test]
fn invoke_of_uncontracted_bound_proc_is_unknown() {
    let members = HashMap::from([(
        "tick".to_string(),
        ResolvedMember::Hook(Some("Elsewhere".to_string())),
    )]);
    let env = InterfaceEnv {
        interfaces: HashMap::from([("Game".to_string(), ResolvedInterface { members })]),
    };
    let items = eval_proc(INVOKER, "P", &env);
    let st = d5_status(&items, &["d0", "d1", "d2", "d3", "d4", "a1", "a2"]);
    let reason = st.expect("an invoke of a proc with no contract must break inout(d5)");
    assert!(reason.contains("indirect or unknown callee"), "reason: {reason}");
}

/// Include direction: a bound proc that clobbers d5 fires, charged to the callee
/// clobber rather than to an unknown callee.
#[test]
fn invoke_of_clobbering_hook_fires() {
    let items = eval_proc(INVOKER, "P", &env_with_bound_hook());
    let st = d5_status(&items, &["d0", "d5"]);
    let reason = st.expect("a d5-clobbering hook must break inout(d5)");
    assert!(!reason.contains("unknown callee"), "charged to the callee's clobber: {reason}");
}

// ---- calls: D1b `[call.input-undefined]` and D1c `[call.live-clobbered]` ----

fn regset(rs: &[&str]) -> BTreeSet<String> {
    rs.iter().map(|s| s.to_string()).collect()
}

/// D1b at the invoke site itself: `Bound` reads d5, P never defines it, so the
/// invoke fires exactly as `jsr Bound` would.
#[test]
fn d1b_checks_the_bound_procs_inputs_at_an_invoke() {
    let src = "module m\n proc P () clobbers(d0-d7) {\n invoke Game.tick\n rts\n }\n";
    let items = eval_proc(src, "P", &env_with_bound_hook());
    let params = BTreeMap::from([("Bound".to_string(), regset(&["d5"]))]);
    let f = check_input_undefined("P", &BTreeSet::new(), &items, &params, &BTreeMap::new(), &BTreeMap::new());
    let got: Vec<(String, String)> = f.into_iter().map(|x| (x.callee, x.reg)).collect();
    assert_eq!(got, vec![("Bound".to_string(), "d5".to_string())]);
}

/// D1b must-def credits the bound proc's unconditional out: `Bound` produces d5,
/// so the later `jbsr Need` (which reads d5) sees it defined.
#[test]
fn d1b_credits_the_bound_procs_out_after_an_invoke() {
    let src = "module m\n proc P () clobbers(d0-d7) {\n invoke Game.tick\n jbsr Need\n rts\n }\n";
    let items = eval_proc(src, "P", &env_with_bound_hook());
    let params = BTreeMap::from([("Need".to_string(), regset(&["d5"]))]);
    let outs = BTreeMap::from([("Bound".to_string(), regset(&["d5"]))]);
    let f = check_input_undefined("P", &BTreeSet::new(), &items, &params, &outs, &BTreeMap::new());
    assert!(f.is_empty(), "Bound's out(d5) defines d5 for Need: {f:?}");
}

/// D1c at an invoke: d3 is held across it, `Bound` clobbers d3, and d3 is read
/// after, so the held value is destroyed.
#[test]
fn d1c_sees_a_live_value_clobbered_by_the_bound_proc() {
    let src = "module m\n proc P () clobbers(d0-d7) {\n moveq #1, d3\n invoke Game.tick\n move.w d3, d4\n rts\n }\n";
    let items = eval_proc(src, "P", &env_with_bound_hook());
    let eff = BTreeMap::from([("Bound".to_string(), RegEffect { top: false, regs: regset(&["d3"]) })]);
    let none: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let f = check_live_clobbered("P", &BTreeSet::new(), &items, &eff, &none, &none);
    let got: Vec<(String, String)> = f.into_iter().map(|x| (x.callee, x.reg)).collect();
    assert_eq!(got, vec![("Bound".to_string(), "d3".to_string())]);
}
