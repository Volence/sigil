//! Contract-grammar v2 §6 — `[proc.dead-save]` (D1d, WARN/perf tier): a
//! §5-verified save/restore pair for a register the bracketed callee PROVABLY
//! preserves. The lint's firing list is the pass-3 dead-save worklist, so the
//! "callee preserves rN" verdict rides the closure's `effective(C)` set — which
//! is `localWrites ∪ callees − VERIFIED preserves` — NEVER a raw declared claim
//! (a worklist pass-3 cuts code from must be built on proof, not trust).
//!
//! A save is dead iff, on EVERY path across its span, the saved register is
//! clobbered by nothing (no direct write; every bracketed call preserves it).
//! One clobbering path anywhere ⇒ the save is needed ⇒ no firing (the safe
//! direction). Shapes mirror the review's named customers (load_object's movem
//! around AllocDynamic, branch-straddled restores).

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::closure::RegEffect;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::preserves::find_dead_saves;
use sigil_frontend_emp::value::Reg;
use sigil_ir::backend::Cpu;
use std::collections::{BTreeMap, BTreeSet};

/// An `effective` map where callee `c` clobbers exactly `clob` (canonical
/// spellings). A register absent from `clob` is thus PRESERVED by `c`.
fn eff(entries: &[(&str, &[&str])]) -> BTreeMap<String, RegEffect> {
    entries
        .iter()
        .map(|(c, clob)| {
            (
                c.to_string(),
                RegEffect {
                    top: false,
                    regs: clob.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>(),
                },
            )
        })
        .collect()
}

/// Eval the first proc and run the dead-save lint with `effective`.
fn run(src: &str, effective: &BTreeMap<String, RegEffect>) -> Vec<(Reg, Vec<String>)> {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "parse: {diags:?}");
    let p = file
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) => Some(p),
            _ => None,
        })
        .expect("a proc");
    let (buf, _d, _n) =
        eval_proc_body(&file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[]);
    let buf = buf.expect("codebuf");
    let mut out: Vec<(Reg, Vec<String>)> = find_dead_saves(&p.name, &buf.items, effective)
        .into_iter()
        .map(|d| (d.reg, d.callees))
        .collect();
    out.sort();
    out
}

fn regs(fires: &[(Reg, Vec<String>)]) -> Vec<Reg> {
    fires.iter().map(|(r, _)| *r).collect()
}

/// Save d1 around a call to C that PRESERVES d1 (d1 ∉ effective(C)) → the save is
/// dead. C clobbers only d0, so d1's movem save/restore is redundant.
#[test]
fn save_around_preserving_call_fires() {
    let f = run(
        "module m\n\
         proc P () clobbers(d2, a0) {\n\
             movem.l d0-d1, -(sp)\n\
             jbsr    C\n\
             movem.l (sp)+, d0-d1\n\
             move.w  d1, d2\n\
             rts\n\
         }\n",
        &eff(&[("C", &["d0"])]),
    );
    assert!(regs(&f).contains(&Reg::D1), "d1 save is dead (C preserves d1): {f:?}");
    // d0 IS clobbered by C → its save is needed → must NOT fire.
    assert!(!regs(&f).contains(&Reg::D0), "d0 save is needed (C clobbers d0): {f:?}");
    // the reported callee is C.
    let d1 = f.iter().find(|(r, _)| *r == Reg::D1).unwrap();
    assert_eq!(d1.1, vec!["C".to_string()], "callee should be C: {:?}", d1.1);
}

/// Save d1 around a call to C that CLOBBERS d1 → the save is needed → no firing.
#[test]
fn save_around_clobbering_call_no_fire() {
    let f = run(
        "module m\n\
         proc P () clobbers(d2, a0) {\n\
             movem.l d0-d1, -(sp)\n\
             jbsr    C\n\
             movem.l (sp)+, d0-d1\n\
             rts\n\
         }\n",
        &eff(&[("C", &["d0", "d1"])]),
    );
    assert!(f.is_empty(), "C clobbers d1 → save needed, no dead-save: {f:?}");
}

/// The saved register is used as SCRATCH inside the span (a direct write between
/// save and restore) → the save is needed → no firing, even though the call
/// preserves it.
#[test]
fn save_with_scratch_use_no_fire() {
    let f = run(
        "module m\n\
         proc P () clobbers(d2, a0) {\n\
             move.l  d1, -(sp)\n\
             jbsr    C\n\
             moveq   #7, d1\n\
             movea.l (sp)+, d1\n\
             rts\n\
         }\n",
        &eff(&[("C", &["d0"])]),
    );
    assert!(f.is_empty(), "d1 used as scratch in span → save needed: {f:?}");
}

/// A save whose restore is branch-straddled (two restore sites, both un-clobbered)
/// fires ONCE (deduped by push site), like load_object's success/alloc_fail pair.
#[test]
fn branch_straddled_restore_fires_once() {
    let f = run(
        "module m\n\
         proc P () clobbers(d2, a0) {\n\
             movem.l d0-d1, -(sp)\n\
             jbsr    C\n\
             bne     .fail\n\
             movem.l (sp)+, d0-d1\n\
             moveq   #0, d2\n\
             rts\n\
         .fail:\n\
             movem.l (sp)+, d0-d1\n\
             moveq   #1, d2\n\
             rts\n\
         }\n",
        &eff(&[("C", &["d0"])]),
    );
    let d1s = f.iter().filter(|(r, _)| *r == Reg::D1).count();
    assert_eq!(d1s, 1, "one dead-save for d1 despite two restore sites: {f:?}");
}

/// If ONE path clobbers the saved register before its restore, the save is NEEDED
/// on that path → NOT dead → no firing (the safe direction for a code-cutting
/// worklist).
#[test]
fn one_clobbering_path_no_fire() {
    let f = run(
        "module m\n\
         proc P () clobbers(d2, a0) {\n\
             move.l  d1, -(sp)\n\
             jbsr    C\n\
             bne     .other\n\
             moveq   #9, d1\n\
             movea.l (sp)+, d1\n\
             rts\n\
         .other:\n\
             movea.l (sp)+, d1\n\
             rts\n\
         }\n",
        &eff(&[("C", &["d0"])]),
    );
    assert!(
        !regs(&f).contains(&Reg::D1),
        "one path clobbers d1 → not dead: {f:?}"
    );
}

/// A hole (callee absent from `effective`) is conservatively treated as
/// clobbering everything → the save is NOT reported dead (never cut code across
/// an unknown callee).
#[test]
fn hole_callee_no_fire() {
    let f = run(
        "module m\n\
         proc P () clobbers(d2, a0) {\n\
             movem.l d0-d1, -(sp)\n\
             jbsr    Unknown\n\
             movem.l (sp)+, d0-d1\n\
             rts\n\
         }\n",
        &eff(&[]),
    );
    assert!(f.is_empty(), "unknown callee → conservative, no dead-save: {f:?}");
}
