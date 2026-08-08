//! The `inout(rN)` facet — threaded-cursor verification (Spec 2026-08-08-inout).
//!
//! An `inout(rN)` register is provided by the caller at entry and read by the
//! caller at exit; PASS-THROUGH (no write) is contract-valid, which is the whole
//! difference from `out`. The exit-side check leaves each declared register in an
//! OK (Entry or Produced) or Broken state and fires on Broken. Every transfer rule
//! is exercised here in BOTH polarities, plus the three mandatory spec probes.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::out_verify::{verify_inout, verify_out, InoutCallees, OutClaim, OutStatus, OutWidth, OutWidths};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{CodeItem, Reg};
use sigil_ir::backend::Cpu;
use std::collections::{BTreeMap, BTreeSet};

/// Eval every proc in `src`, returning name → evaluated CodeItems.
fn eval_all(src: &str) -> BTreeMap<String, Vec<CodeItem>> {
    let (file, diags) = parse_str(src);
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {diags:?}");
    let mut out = BTreeMap::new();
    let mut counter = 0u32;
    for item in &file.items {
        if let Item::Proc(p) = item {
            let (buf, _d, next) = eval_proc_body(
                &file, &p.name, &p.params, &p.body, p.span, counter, Cpu::M68000, &[],
                &sigil_frontend_emp::contract::InterfaceEnv::empty(),
            );
            counter = next;
            if let Some(buf) = buf {
                out.insert(p.name.clone(), buf.items);
            }
        }
    }
    out
}

fn names(entries: &[(&str, &[Reg])]) -> BTreeMap<String, BTreeSet<String>> {
    entries
        .iter()
        .map(|(n, regs)| (n.to_string(), regs.iter().map(|r| r.to_string()).collect()))
        .collect()
}

/// d5 declared `u16` — the own-width map the sprite-cursor procs use.
fn d5_u16() -> BTreeMap<String, OutClaim> {
    BTreeMap::from([("d5".to_string(), OutClaim::exact(OutWidth::W))])
}

/// Run `verify_inout` for `reg` on `proc`'s body with the given callee maps, and
/// return whether it VERIFIED (non-Broken on every exit).
#[allow(clippy::too_many_arguments)]
fn inout_ok(
    src: &str,
    proc: &str,
    reg: Reg,
    own: &BTreeMap<String, OutClaim>,
    uncond_out: &BTreeMap<String, BTreeSet<String>>,
    inout: &BTreeMap<String, BTreeSet<String>>,
    cond_out: &BTreeMap<String, Vec<(String, String)>>,
    effective_clobbers: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    let all = eval_all(src);
    let items = all.get(proc).unwrap_or_else(|| panic!("no proc {proc}"));
    let no_callee_widths = BTreeMap::new();
    let statuses = verify_inout(
        items,
        &[reg],
        OutWidths { own, callees: &no_callee_widths },
        InoutCallees { uncond_out, inout, cond_out, effective_clobbers },
        None,
        &BTreeSet::new(),
    );
    statuses.get(&reg).expect("status for reg").is_none()
}

fn empty_maps() -> (BTreeMap<String, BTreeSet<String>>, BTreeMap<String, Vec<(String, String)>>) {
    (BTreeMap::new(), BTreeMap::new())
}

// -- Rule 1: a full-width write produces (repairs); a narrower write breaks. ----

#[test]
fn rule1_full_width_write_verifies_and_narrow_breaks() {
    let (m, c) = empty_maps();
    // addq.w on a u16 claim — full width → OK.
    let full = "module m\n proc P (d5: u16) clobbers() inout(d5: u16) {\n addq.w #1, d5\n rts\n }\n";
    assert!(inout_ok(full, "P", Reg::D5, &d5_u16(), &m, &m, &c, &m), "addq.w should verify");
    // addq.b on a u16 claim — partial → Broken. (MANDATORY PROBE (a).)
    let part = "module m\n proc P (d5: u16) clobbers() inout(d5: u16) {\n addq.b #1, d5\n rts\n }\n";
    assert!(!inout_ok(part, "P", Reg::D5, &d5_u16(), &m, &m, &c, &m), "addq.b must break a u16 inout");
}

// -- Non-vacuity (MANDATORY PROBE (c)): pass-through PASSES inout, FAILS out. ----

#[test]
fn passthrough_verifies_under_inout_but_not_under_out() {
    let src = "module m\n proc P (d5: u16) clobbers() inout(d5: u16) {\n rts\n }\n";
    let (m, c) = empty_maps();
    // inout: Entry value handed back → OK.
    assert!(inout_ok(src, "P", Reg::D5, &d5_u16(), &m, &m, &c, &m), "pass-through must verify under inout");
    // out: the SAME body, checked as `out(d5)`, is NOT produced → Unverified. This
    // is what proves inout does not share out's path (earned-back param seeding).
    let all = eval_all(src);
    let items = all.get("P").unwrap();
    let st = verify_out(items, &[Reg::D5], &[], &m, &c, None, &BTreeSet::new(), OutWidths::bare())
        .remove(&Reg::D5)
        .unwrap();
    assert!(matches!(st, OutStatus::Unverified(_)), "the same body must FAIL under out()");
}

// -- Rule 3: a callee's unconditional out reproduces (repairs a prior break). ---

#[test]
fn rule3_uncond_out_callee_repairs() {
    // d5 broken by addq.b, then a callee with out(d5) reproduces it → OK.
    let src = "module m\n proc P (d5: u16) clobbers(d0) inout(d5: u16) {\n addq.b #1, d5\n jbsr Maker\n rts\n }\n";
    let (inout, c) = empty_maps();
    let uncond = names(&[("Maker", &[Reg::D5])]);
    let clob = names(&[("Maker", &[Reg::D5])]); // known callee
    assert!(inout_ok(src, "P", Reg::D5, &d5_u16(), &uncond, &inout, &c, &clob), "uncond-out call must repair");
    // WITHOUT the reproducing call (Maker unknown / no out), the break stands.
    let (m, _) = empty_maps();
    assert!(!inout_ok(src, "P", Reg::D5, &d5_u16(), &m, &inout, &c, &m), "no repair ⇒ broken stands");
}

// -- Rule 4: a callee's CONDITIONAL out is not production → Broken. -------------

#[test]
fn rule4_cond_out_callee_breaks() {
    let src = "module m\n proc P (d5: u16) clobbers(d0) inout(d5: u16) {\n jbsr CondMaker\n rts\n }\n";
    let (uncond, inout) = (BTreeMap::new(), BTreeMap::new());
    let cond: BTreeMap<String, Vec<(String, String)>> =
        BTreeMap::from([("CondMaker".to_string(), vec![("d5".to_string(), "eq".to_string())])]);
    let clob = names(&[("CondMaker", &[])]); // known, does not clobber d5, but cond-out
    assert!(!inout_ok(src, "P", Reg::D5, &d5_u16(), &uncond, &inout, &cond, &clob), "cond-out must break");
}

// -- Rule 5: a callee that CLOBBERS the register → Broken. (PROBE (b).) ---------

#[test]
fn rule5_clobbering_callee_breaks() {
    let src = "module m\n proc P (d5: u16) clobbers(d0) inout(d5: u16) {\n jbsr Clobberer\n rts\n }\n";
    let (uncond, inout, c) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
    let clob = names(&[("Clobberer", &[Reg::D5])]);
    assert!(!inout_ok(src, "P", Reg::D5, &d5_u16(), &uncond, &inout, &c, &clob), "clobbering callee must break");
}

// -- Rule 6: a callee's inout threads through UNCHANGED (composition). ----------

#[test]
fn rule6_inout_callee_is_state_preserving() {
    // Pass-through THROUGH an inout helper, no local write → OK (composition PASS).
    let thru = "module m\n proc P (d5: u16) clobbers(d0) inout(d5: u16) {\n jbsr Helper\n rts\n }\n";
    let (uncond, c) = (BTreeMap::new(), BTreeMap::new());
    let inout = names(&[("Helper", &[Reg::D5])]);
    let clob = names(&[("Helper", &[Reg::D5])]); // present ⇒ known; d5 IS in its clobbers-equivalent, but the inout rule wins first
    assert!(inout_ok(thru, "P", Reg::D5, &d5_u16(), &uncond, &inout, &c, &clob), "pass-through via inout helper must verify");
    // But an inout callee does NOT REPAIR a prior break (unchanged, not produced).
    let broke = "module m\n proc P (d5: u16) clobbers(d0) inout(d5: u16) {\n addq.b #1, d5\n jbsr Helper\n rts\n }\n";
    assert!(!inout_ok(broke, "P", Reg::D5, &d5_u16(), &uncond, &inout, &c, &clob), "inout helper must not repair a break");
}

// -- Rule 7: a callee that PRESERVES / does not touch the register → unchanged. -

#[test]
fn rule7_preserving_callee_is_unchanged_and_unknown_breaks() {
    let src = "module m\n proc P (d5: u16) clobbers(d0) inout(d5: u16) {\n jbsr Other\n rts\n }\n";
    let (uncond, inout, c) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
    // KNOWN callee (present) whose clobber set excludes d5 → preserved → OK.
    let known_preserves = names(&[("Other", &[Reg::D0])]);
    assert!(inout_ok(src, "P", Reg::D5, &d5_u16(), &uncond, &inout, &c, &known_preserves), "preserving callee ⇒ unchanged");
    // UNKNOWN callee (absent from the effective-clobber map) → Broken (scope guard).
    let empty = BTreeMap::new();
    assert!(!inout_ok(src, "P", Reg::D5, &d5_u16(), &uncond, &inout, &c, &empty), "unknown callee must break");
}
