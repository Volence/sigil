//! Contract-grammar v2 §6 / G4 — the caller-side input + liveness checks over
//! the SHARED lightweight CFG (`flag_check::Cfg`, real joins, not straight-line):
//!
//! - `[call.input-undefined]` (D1b): a register param of the callee has no
//!   reaching definition at the call site on SOME path (a must-def all-paths
//!   check). Evidence: the AnimateSprite d3/DUR_DYNAMIC garbage bug.
//! - `[call.live-clobbered]` (D1c): a value defined before the call and read
//!   after it, held in a register the callee EFFECTIVELY clobbers (post-preserves
//!   subtraction) — pass-3's seatbelt.
//!
//! Exercised end-to-end from `.emp` through `eval_proc_body`, so the checks run
//! against the real evaluator's CodeBuf exactly as the corpus walk does.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::calls::{check_input_undefined, check_live_clobbered};
use sigil_frontend_emp::closure::RegEffect;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Reg;
use sigil_ir::backend::Cpu;
use std::collections::{BTreeMap, BTreeSet};

/// Eval the first proc in `src`; return `(proc-name, caller-params, CodeItems)`.
fn eval_first(src: &str) -> (String, BTreeSet<String>, Vec<sigil_frontend_emp::value::CodeItem>) {
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
    let params: BTreeSet<String> = p
        .params
        .iter()
        .filter_map(|(n, _, _)| Reg::from_name(n).map(|r| r.to_string()))
        .collect();
    (p.name.clone(), params, buf.items)
}

fn regset(regs: &[&str]) -> BTreeSet<String> {
    regs.iter().map(|s| s.to_string()).collect()
}

// ===========================================================================
// [call.input-undefined] (D1b)
// ===========================================================================

/// Run the input-undefined check with `callee` declaring `callee_params` inputs.
fn run_input(src: &str, callee: &str, callee_params: &[&str]) -> Vec<String> {
    run_input_out(src, callee, callee_params, &[])
}

/// Run the input-undefined check where `callee` declares `callee_params` inputs
/// AND `callee_out` unconditional outputs (credited as definitions at each call).
fn run_input_out(
    src: &str,
    callee: &str,
    callee_params: &[&str],
    callee_out: &[&str],
) -> Vec<String> {
    let (name, params, items) = eval_first(src);
    let mut pmap: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    pmap.insert(callee.to_string(), regset(callee_params));
    let mut omap: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    omap.insert(callee.to_string(), regset(callee_out));
    check_input_undefined(&name, &params, &items, &pmap, &omap)
        .into_iter()
        .map(|f| f.reg)
        .collect()
}

/// A callee reads d3 as input, but the caller never defines d3 before the call —
/// the AnimateSprite d3/DUR_DYNAMIC bug class. Fires on d3.
#[test]
fn undefined_input_fires() {
    let f = run_input(
        "module m\n\
         proc P () clobbers(d0) {\n\
             moveq #0, d0\n\
             jbsr Anim\n\
             rts\n\
         }\n",
        "Anim",
        &["d3"],
    );
    assert_eq!(f, vec!["d3".to_string()], "d3 never defined before the call: {f:?}");
}

/// The input is written before the call on the only path — defined, no firing.
#[test]
fn input_defined_before_call_passes() {
    let f = run_input(
        "module m\n\
         proc P () clobbers(d0/d3) {\n\
             moveq #7, d3\n\
             jbsr Anim\n\
             rts\n\
         }\n",
        "Anim",
        &["d3"],
    );
    assert!(f.is_empty(), "d3 defined by moveq before the call: {f:?}");
}

/// The input is the CALLER's own param — defined on entry, no firing. (This is
/// exactly what the `// In:`→param retrofit turns on. A typed param today; the
/// bare-register form the retrofit uses lands with the retrofit sweep.)
#[test]
fn caller_param_is_defined_on_entry_passes() {
    let f = run_input(
        "module m\n\
         proc P (d3: u16) clobbers(d0) {\n\
             jbsr Anim\n\
             rts\n\
         }\n",
        "Anim",
        &["d3"],
    );
    assert!(f.is_empty(), "d3 is the caller's own declared input — defined on entry: {f:?}");
}

/// The input is defined on ONE branch of an if but not the other; the call after
/// the join sees an undefined path → fires. (Why the CFG needs real joins.)
#[test]
fn input_defined_on_only_one_branch_fires() {
    let f = run_input(
        "module m\n\
         proc P () clobbers(d0/d3) {\n\
             moveq #0, d0\n\
             tst.w d0\n\
             beq .skip\n\
             moveq #1, d3\n\
         .skip:\n\
             jbsr Anim\n\
             rts\n\
         }\n",
        "Anim",
        &["d3"],
    );
    assert_eq!(f, vec!["d3".to_string()], "the .skip path leaves d3 undefined: {f:?}");
}

/// The input is defined on BOTH branches before the join+call → defined on every
/// path, no firing.
#[test]
fn input_defined_on_both_branches_passes() {
    let f = run_input(
        "module m\n\
         proc P () clobbers(d0/d3) {\n\
             moveq #0, d0\n\
             tst.w d0\n\
             beq .zero\n\
             moveq #1, d3\n\
             bra .join\n\
         .zero:\n\
             moveq #2, d3\n\
         .join:\n\
             jbsr Anim\n\
             rts\n\
         }\n",
        "Anim",
        &["d3"],
    );
    assert!(f.is_empty(), "d3 defined on both branches before the call: {f:?}");
}

/// A callee with no known params is never checked (nothing to be undefined).
#[test]
fn callee_without_params_is_not_checked() {
    let f = run_input(
        "module m\n\
         proc P () clobbers(d0) {\n\
             jbsr Plain\n\
             rts\n\
         }\n",
        "Plain",
        &[],
    );
    assert!(f.is_empty(), "Plain has no input params: {f:?}");
}

// ===========================================================================
// [call.live-clobbered] (D1c)
// ===========================================================================

fn effect(regs: &[&str]) -> RegEffect {
    RegEffect { top: false, regs: regset(regs) }
}

/// Run the live-clobbered check with `callee` clobbering `clob` (effective) and
/// producing `out`. Returns the firing register names.
fn run_live(src: &str, callee: &str, clob: &[&str], out: &[&str]) -> Vec<String> {
    let (name, params, items) = eval_first(src);
    let mut eff: BTreeMap<String, RegEffect> = BTreeMap::new();
    eff.insert(callee.to_string(), effect(clob));
    let mut outs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    outs.insert(callee.to_string(), regset(out));
    check_live_clobbered(&name, &params, &items, &eff, &outs)
        .into_iter()
        .map(|f| f.reg)
        .collect()
}

/// A value written before the call, held in a register the callee clobbers, and
/// read after — the caller-saved-register bug. Fires. (pass-3's seatbelt.)
#[test]
fn live_value_clobbered_across_call_fires() {
    let f = run_live(
        "module m\n\
         proc P () clobbers(d0-d1/a0) {\n\
             lea Table, a0\n\
             move.l (a0), d0\n\
             jbsr Munge\n\
             move.l (a0), d1\n\
             rts\n\
         }\n",
        "Munge",
        &["a0"],
        &[],
    );
    assert_eq!(f, vec!["a0".to_string()], "a0 held across Munge which clobbers it: {f:?}");
}

/// The value is REDEFINED after the call before it is read → not live across the
/// call, no firing.
#[test]
fn redefined_after_call_passes() {
    let f = run_live(
        "module m\n\
         proc P () clobbers(d0/a0) {\n\
             lea Table, a0\n\
             jbsr Munge\n\
             lea Other, a0\n\
             move.l (a0), d0\n\
             rts\n\
         }\n",
        "Munge",
        &["a0"],
        &[],
    );
    assert!(f.is_empty(), "a0 rebuilt after the call before use: {f:?}");
}

/// The callee PRESERVES the register (it is not in the effective clobber set), so
/// holding a value across it is fine — no firing. This is the free consequence of
/// keying off the closure's effective set (verified preserves already subtracted).
#[test]
fn callee_preserves_the_register_passes() {
    let f = run_live(
        "module m\n\
         proc P () clobbers(d0/a0) {\n\
             lea Table, a0\n\
             jbsr Munge\n\
             move.l (a0), d0\n\
             rts\n\
         }\n",
        "Munge",
        &["d0"], // Munge clobbers d0, NOT a0 — a0 survives
        &[],
    );
    assert!(f.is_empty(), "Munge preserves a0 (a0 ∉ effective): {f:?}");
}

/// The value is set up purely as the call's ARGUMENT and never read afterward —
/// the callee consuming its own input is fine, no firing.
#[test]
fn value_not_read_after_call_passes() {
    let f = run_live(
        "module m\n\
         proc P () clobbers(d0/a0) {\n\
             lea Table, a0\n\
             jbsr Munge\n\
             rts\n\
         }\n",
        "Munge",
        &["a0"],
        &[],
    );
    assert!(f.is_empty(), "a0 is an argument, never read after: {f:?}");
}

/// The clobbered register is the callee's declared OUTPUT: the caller reads the
/// callee's RESULT, not a stale held value — no firing (out excluded from the
/// clobber-of-a-held-value set).
#[test]
fn callee_output_register_is_not_a_held_value_passes() {
    let f = run_live(
        "module m\n\
         proc P () clobbers(d0/a1) {\n\
             lea Table, a1\n\
             jbsr Alloc\n\
             move.l (a1), d0\n\
             rts\n\
         }\n",
        "Alloc",
        &["a1"], // Alloc clobbers a1 ...
        &["a1"], // ... but a1 is its OUTPUT — reading it is the result, not a bug
    );
    assert!(f.is_empty(), "a1 is Alloc's output result: {f:?}");
}

/// The value is live-and-clobbered on only ONE path after the call → still fires
/// (the bug exists on that path; why the CFG needs joins).
#[test]
fn live_on_one_path_fires() {
    let f = run_live(
        "module m\n\
         proc P () clobbers(d0-d1/a0) {\n\
             lea Table, a0\n\
             jbsr Munge\n\
             tst.w d0\n\
             beq .skip\n\
             move.l (a0), d1\n\
         .skip:\n\
             rts\n\
         }\n",
        "Munge",
        &["a0", "d0"],
        &[],
    );
    assert!(f.contains(&"a0".to_string()), "a0 read on the non-skip path: {f:?}");
}

/// Two consecutive clobbering calls, value read after the second: the SECOND
/// call is the one that destroys the value reaching the read — only it fires.
/// The first call's liveness is killed by the second (an intervening clobbering
/// call is a redefine).
#[test]
fn intervening_clobbering_call_ends_liveness() {
    let (name, params, items) = eval_first(
        "module m\n\
         proc P () clobbers(d0/a0) {\n\
             lea Table, a0\n\
             jbsr First\n\
             jbsr Second\n\
             move.l (a0), d0\n\
             rts\n\
         }\n",
    );
    let mut eff: BTreeMap<String, RegEffect> = BTreeMap::new();
    eff.insert("First".to_string(), effect(&["a0"]));
    eff.insert("Second".to_string(), effect(&["a0"]));
    let outs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let f = check_live_clobbered(&name, &params, &items, &eff, &outs);
    let callees: Vec<&str> = f.iter().map(|x| x.callee.as_str()).collect();
    assert_eq!(callees, vec!["Second"], "only the second (last) clobber before the read fires: {f:?}");
}

/// The corpus pattern: the caller SAVES the register (`movem.l d5/d7,-(sp)`)
/// before a clobbering call and RESTORES it after — so the value across the call
/// is correctly preserved by the CALLER. Not a bug; must NOT fire. (A movem load
/// redefines its reglist; the restore prunes the liveness. This is the exact
/// tile_cache.emp `movem.l (sp)+, d5/d7` around a DecompressBlock/FillColumn
/// call.)
#[test]
fn movem_save_restore_around_clobbering_call_passes() {
    let (name, params, items) = eval_first(
        "module m\n\
         proc P () clobbers(d0-d7/a0) {\n\
             move.l Foo, d5\n\
             move.l Bar, d7\n\
             movem.l d5/d7, -(sp)\n\
             jbsr Munge\n\
             movem.l (sp)+, d5/d7\n\
             add.l d5, d7\n\
             rts\n\
         }\n",
    );
    let mut eff: BTreeMap<String, RegEffect> = BTreeMap::new();
    eff.insert("Munge".to_string(), effect(&["d5", "d7"]));
    let outs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let f = check_live_clobbered(&name, &params, &items, &eff, &outs);
    assert!(f.is_empty(), "d5/d7 saved+restored across Munge — not live-clobbered: {f:?}");
}

/// A partial save: only d5 is saved/restored, d7 is NOT — d7 held across the same
/// clobbering call still fires (the movem restore only redefines what it lists).
#[test]
fn partial_movem_save_fires_only_for_the_unsaved_register() {
    let (name, params, items) = eval_first(
        "module m\n\
         proc P () clobbers(d0-d7/a0) {\n\
             move.l Foo, d5\n\
             move.l Bar, d7\n\
             movem.l d5, -(sp)\n\
             jbsr Munge\n\
             movem.l (sp)+, d5\n\
             move.l d7, (a0)\n\
             move.l d5, (a0)\n\
             rts\n\
         }\n",
    );
    let mut eff: BTreeMap<String, RegEffect> = BTreeMap::new();
    eff.insert("Munge".to_string(), effect(&["d5", "d7"]));
    let outs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let f = check_live_clobbered(&name, &params, &items, &eff, &outs);
    let regs: Vec<&str> = f.iter().map(|x| x.reg.as_str()).collect();
    assert_eq!(regs, vec!["d7"], "only the unsaved d7 fires (d5 restored): {f:?}");
}

/// An intervening call that OUTPUTS the register (declares `out(R)`) redefines it
/// — it produces a fresh value — so a read after that second call reads the new
/// value, not the first call's clobbered one. The first call must NOT fire. (The
/// corpus Tile_Cache_Fill → VSlide(clobbers d0) → FillRow(out d0) → tst.w d0
/// pattern.)
#[test]
fn intervening_out_call_redefines_and_suppresses() {
    let (name, params, items) = eval_first(
        "module m\n\
         proc P () clobbers(d0-d1/a0) {\n\
             moveq #2, d0\n\
             jbsr VSlide\n\
             jbsr FillRow\n\
             tst.w d0\n\
             rts\n\
         }\n",
    );
    let mut eff: BTreeMap<String, RegEffect> = BTreeMap::new();
    eff.insert("VSlide".to_string(), effect(&["d0"]));
    eff.insert("FillRow".to_string(), effect(&["d0"])); // FillRow writes d0 ...
    let mut outs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    outs.insert("FillRow".to_string(), regset(&["d0"])); // ... as its declared OUTPUT
    let f = check_live_clobbered(&name, &params, &items, &eff, &outs);
    assert!(f.is_empty(), "FillRow's out(d0) redefines d0 before the read — VSlide must not fire: {f:?}");
}

/// An unbounded (⊤) effective set clobbers every register — a held value read
/// afterward fires. (⊤ only arises from an unbounded indirect; here it is
/// exercised directly.)
#[test]
fn top_effective_clobbers_held_value_fires() {
    let (name, params, items) = eval_first(
        "module m\n\
         proc P () clobbers(d0/a0) {\n\
             lea Table, a0\n\
             jbsr Wild\n\
             move.l (a0), d0\n\
             rts\n\
         }\n",
    );
    let mut eff: BTreeMap<String, RegEffect> = BTreeMap::new();
    eff.insert("Wild".to_string(), RegEffect { top: true, regs: BTreeSet::new() });
    let outs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let f = check_live_clobbered(&name, &params, &items, &eff, &outs);
    assert!(f.iter().any(|x| x.reg == "a0"), "⊤ clobbers a0 held across it: {f:?}");
}

// --- D1b: a callee's UNCONDITIONAL out() credits a definition (must-def) ------
//
// A value produced by one call's plain `out(rN)` and passed to the next call is
// must-defined — D1b must NOT fire. (A conditional `out(rN if cc)` is excluded
// from `callee_out` and is NOT credited — the sound half; edge-sensitive
// success-edge crediting is deferred.)
#[test]
fn callee_unconditional_out_credits_definition() {
    let src = "module m\n\
        proc P () clobbers(d0) {\n\
            jbsr Producer\n\
            jbsr Consumer\n\
            rts\n\
        }\n";
    let (name, params, items) = eval_first(src);
    let mut pmap: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    pmap.insert("Consumer".to_string(), regset(&["d0"]));

    // WITHOUT Producer's out credited: d0 is undefined at the Consumer call.
    let empty: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let without: Vec<String> = check_input_undefined(&name, &params, &items, &pmap, &empty)
        .into_iter()
        .map(|f| f.reg)
        .collect();
    assert_eq!(without, vec!["d0".to_string()], "no out credit → d0 undefined, got {without:?}");

    // WITH Producer's unconditional out(d0): d0 is defined → no firing.
    let mut omap: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    omap.insert("Producer".to_string(), regset(&["d0"]));
    let with: Vec<String> = check_input_undefined(&name, &params, &items, &pmap, &omap)
        .into_iter()
        .map(|f| f.reg)
        .collect();
    assert!(with.is_empty(), "Producer out(d0) credited → no firing, got {with:?}");
}
