//! Contract-grammar v2 — the whole-corpus contract walk end to end: parse
//! synthetic `.emp` modules → `analyze_corpus` → closure + firings. Exercises
//! the AST→ProcNode wiring the pure closure tests can't (call-edge extraction,
//! indirect-site bounds, extern leaves, the §11 Q4 collision).

use sigil_frontend_emp::corpus_contracts::analyze_corpus;
use sigil_frontend_emp::parse_str;

/// Parse each source into a `File` (demanding clean parse) and analyze.
fn analyze(srcs: &[&str]) -> sigil_frontend_emp::corpus_contracts::ContractReport {
    let files: Vec<_> = srcs
        .iter()
        .map(|s| {
            let (f, diags) = parse_str(s);
            assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
            f
        })
        .collect();
    analyze_corpus(&files)
}

/// A firing on `(proc, reg)` is present.
fn fires(r: &sigil_frontend_emp::corpus_contracts::ContractReport, proc: &str, reg: &str) -> bool {
    r.firings.iter().any(|f| f.proc == proc && f.reg.as_deref() == Some(reg))
}

/// A proc that writes a register outside its declared `clobbers` fires (the
/// transitive lint subsumes the local one for a direct write).
#[test]
fn direct_under_declaration_fires_over_corpus() {
    let r = analyze(&[
        "module m\nproc P () clobbers(d0) {\n moveq #0, d0\n moveq #1, d7\n rts }\n",
    ]);
    assert!(fires(&r, "P", "d7"), "firings: {:?}", r.firings);
    assert!(!fires(&r, "P", "d0"), "d0 is declared, must not fire");
}

/// The number of `[call.flag-result-unused]` firings on `proc` calling `callee`.
fn flag_fires(
    r: &sigil_frontend_emp::corpus_contracts::ContractReport,
    proc: &str,
    callee: &str,
) -> usize {
    r.flag_firings.iter().filter(|f| f.proc == proc && f.callee == callee).count()
}

/// An extern declaring `out(carry: dropped)` whose caller CONSUMES the carry
/// (`bcs`) produces no flag firing — the wired end-to-end happy path (contract
/// from a decl, CFG over the caller's evaluated body).
#[test]
fn flag_result_consumed_over_corpus_passes() {
    let r = analyze(&[
        "module m\n\
         extern proc Queue (d1) clobbers(d0) out(carry: dropped)\n\
         proc Caller () clobbers(d0-d1) {\n\
             jbsr Queue\n\
             bcs .done\n\
             moveq #0, d0\n\
         .done:\n\
             rts\n\
         }\n",
    ]);
    assert_eq!(flag_fires(&r, "Caller", "Queue"), 0, "flag firings: {:?}", r.flag_firings);
}

/// The same extern whose caller DROPS the carry (overwrites CC and returns)
/// fires `[call.flag-result-unused]` — the Palette_Dirty / load_art bug class,
/// caught through the whole-corpus wiring.
#[test]
fn flag_result_dropped_over_corpus_fires() {
    let r = analyze(&[
        "module m\n\
         extern proc Queue (d1) clobbers(d0) out(carry: dropped)\n\
         proc Caller () clobbers(d0-d1) {\n\
             jbsr Queue\n\
             moveq #0, d0\n\
             rts\n\
         }\n",
    ]);
    assert_eq!(flag_fires(&r, "Caller", "Queue"), 1, "flag firings: {:?}", r.flag_firings);
}

/// `@discards(dropped)` on the call is the explicit opt-out — no firing even
/// though the carry is dropped.
#[test]
fn flag_result_discarded_over_corpus_passes() {
    let r = analyze(&[
        "module m\n\
         extern proc Queue (d1) clobbers(d0) out(carry: dropped)\n\
         proc Caller () clobbers(d0-d1) {\n\
             jbsr Queue @discards(dropped)\n\
             moveq #0, d0\n\
             rts\n\
         }\n",
    ]);
    assert_eq!(flag_fires(&r, "Caller", "Queue"), 0, "flag firings: {:?}", r.flag_firings);
}

/// A caller with a tight contract that CALLS a scribbler is charged the
/// scribbler's writes transitively (the whole point of §1).
#[test]
fn transitive_callee_leak_fires_over_corpus() {
    let r = analyze(&[
        "module m\n\
         proc Caller () clobbers(d0) {\n moveq #0, d0\n jbsr Scribbler\n rts }\n\
         proc Scribbler () clobbers(d1) {\n moveq #1, d1\n rts }\n",
    ]);
    // Caller declares only d0 but transitively clobbers d1 via Scribbler.
    assert!(fires(&r, "Caller", "d1"), "firings: {:?}", r.firings);
}

/// An `extern proc` leaf charges its declared clobbers to its callers (§3): a
/// caller of `VSync_Wait () clobbers(d0)` that declares `clobbers()` fires d0.
#[test]
fn extern_leaf_charges_callers() {
    let r = analyze(&[
        "module m\n\
         extern proc VSync_Wait () clobbers(d0)\n\
         proc Frame () clobbers() {\n jbsr VSync_Wait\n rts }\n",
    ]);
    assert!(fires(&r, "Frame", "d0"), "firings: {:?}", r.firings);
    assert_eq!(r.extern_count, 1);
}

/// A BOUNDED indirect dispatch (`jsr (a1) as HBlankHandler`) charges only the
/// bound's clobbers — NOT ⊤ — so a proc declaring exactly that set does not fire.
#[test]
fn bounded_indirect_is_not_top() {
    let r = analyze(&[
        "module m\n\
         type HBlankHandler = proc () clobbers(d0, d1, a0)\n\
         proc Dispatch () clobbers(d0, d1, a0) {\n jsr (a1) as HBlankHandler\n rts }\n",
    ]);
    assert!(r.firings.is_empty(), "bounded dispatch should not fire: {:?}", r.firings);
    assert_eq!(r.contract_type_count, 1);
}

/// An UNBOUNDED indirect dispatch (`jsr (a1)` with no `as`) makes the proc's
/// effect ⊤ — a bounded `clobbers` contract on it fires unbounded.
#[test]
fn unbounded_indirect_fires_unbounded() {
    let r = analyze(&[
        "module m\nproc Dispatch () clobbers(d0) {\n jsr (a1)\n rts }\n",
    ]);
    assert!(
        r.firings.iter().any(|f| f.proc == "Dispatch" && f.unbounded),
        "firings: {:?}",
        r.firings
    );
}

/// A preserves-only contract type bounds clobbers to everything-not-preserved:
/// `ObjRoutine preserves(a0, d7)` lets a target clobber the rest, so a dispatcher
/// declaring the full register file minus nothing does not fire, but a0/d7 stay
/// protected (not charged).
#[test]
fn preserves_only_type_bounds_complement() {
    let r = analyze(&[
        "module m\n\
         type ObjRoutine = proc (a0: *Sst) preserves(a0, d7)\n\
         proc Run () clobbers(d0-d6/a1-a6) {\n jsr (a1) as ObjRoutine\n rts }\n",
    ]);
    // a0 and d7 are preserved by the bound, so they are never charged; the rest
    // (d0-d6/a1-a6) is exactly declared → no firing.
    assert!(r.firings.is_empty(), "firings: {:?}", r.firings);
}

/// A name declared BOTH `extern proc` and `proc` collides (§11 Q4).
#[test]
fn extern_proc_collision_flagged() {
    let r = analyze(&[
        "module a\nextern proc Shared () clobbers(d0)\n",
        "module b\nproc Shared () clobbers(d0) {\n rts }\n",
    ]);
    assert!(
        r.extern_collisions.iter().any(|(n, _)| n == "Shared"),
        "collisions: {:?}",
        r.extern_collisions
    );
}

/// An `extern proc` with an `out` register charges that register to its callers
/// too — an out result is WRITTEN by the callee (the S4LZ in-out cursor case),
/// so a caller relying on it across the call is wrong. The extern leaf's
/// effective set is clobbers ∪ out.
#[test]
fn extern_out_register_charges_callers() {
    let r = analyze(&[
        "module m\n\
         extern proc Decompress (a0, a1) clobbers(d0) out(a1)\n\
         proc Caller () clobbers(d0) {\n jbsr Decompress\n rts }\n",
    ]);
    // Caller is charged a1 (the extern's out cursor) but declares only d0.
    assert!(fires(&r, "Caller", "a1"), "firings: {:?}", r.firings);
}

/// Declared + movem-VERIFIED preserves is subtracted (the D2.32 fast path): a
/// proc that writes a0/a1 but movem-saves/restores them, declaring
/// preserves(a0/a1), fires nothing — the registers do not escape it.
#[test]
fn declared_verified_preserves_subtracts_over_corpus() {
    let r = analyze(&[
        "module m\n\
         proc P () clobbers() preserves(a0/a1) {\n\
             movem.l a0-a1, -(sp)\n\
             lea Foo, a0\n\
             lea Bar, a1\n\
             movem.l (sp)+, a0-a1\n\
             rts }\n",
    ]);
    assert!(r.firings.is_empty(), "verified preserves must subtract: {:?}", r.firings);
}

/// Declared but UNVERIFIABLE preserves does NOT subtract: an individual-push
/// save (no movem pair) leaves the D2.32 slice unable to prove preservation, so
/// the register stays in `effective` and fires (and the declared preserves is
/// itself a D2.32 error at its own site — subtracting on an unproven claim would
/// be unsound). This is the row-1030 individual-push class → G3.
#[test]
fn verified_individual_push_preserves_subtracts() {
    // The G3 upgrade (§5): individual-push preserves is now VERIFIABLE, so a0 is
    // subtracted from `effective` and does NOT fire — the AllocDynamic shape. (In
    // G1/G2 the D2.32 movem-only slice could not verify this; it fired.)
    let r = analyze(&[
        "module m\n\
         proc P () clobbers() preserves(a0) {\n\
             move.l a0, -(sp)\n\
             lea Foo, a0\n\
             movea.l (sp)+, a0\n\
             rts }\n",
    ]);
    assert!(
        !fires(&r, "P", "a0"),
        "individual-push preserves is §5-verified → a0 subtracted → must NOT fire: {:?}",
        r.firings
    );
}

#[test]
fn genuinely_unverifiable_preserves_does_not_subtract() {
    // A declared preserves whose proof BAILS (computed sp) is unverifiable → not
    // subtracted → a0 still fires. (A wrong contract earns [proc.preserves-
    // unverifiable] at lowering; the closure conservatively keeps a0.)
    let r = analyze(&[
        "module m\n\
         proc P () clobbers() preserves(a0) {\n\
             move.l a0, -(sp)\n\
             lea Foo, a0\n\
             adda.w #4, sp\n\
             movea.l (sp)+, a0\n\
             rts }\n",
    ]);
    assert!(
        fires(&r, "P", "a0"),
        "an unverifiable (bailed) preserves must NOT subtract → a0 fires: {:?}",
        r.firings
    );
}

// === D1b WARN→ERROR flip — the gate's teeth, hermetic ======================

/// A `[call.input-undefined]` firing on `(proc, callee, reg)` is present.
fn input_fires(
    r: &sigil_frontend_emp::corpus_contracts::ContractReport,
    proc: &str,
    callee: &str,
    reg: &str,
) -> bool {
    r.input_firings
        .iter()
        .any(|f| f.proc == proc && f.callee == callee && f.reg == reg)
}

/// FLIP-GATE RED-TEST (brief §2.6): a caller invoking a callee whose register
/// param is UNDEFINED on the path produces a `[call.input-undefined]` firing — so
/// the ERROR gate (`input_firings` empty) would REJECT it. This is the synthetic
/// undefined-input the corpus gate is a permanent absence-of.
#[test]
fn flip_gate_rejects_undefined_input() {
    let r = analyze(&[
        "module m\n\
         proc Callee (d0: u16) clobbers() { rts }\n\
         proc Caller () clobbers() {\n\
             jbsr Callee\n\
             rts }\n",
    ]);
    assert!(
        input_fires(&r, "Caller", "Callee", "d0"),
        "d0 is Callee's param, undefined in Caller → D1b must fire: {:?}",
        r.input_firings
    );
}

/// VERIFIED-CREDIT IS LOAD-BEARING (brief §2.2, the FindStagedBlock shape as a
/// permanent regression): `Liar` DECLARES `out(d0)` but only produces d0 on one
/// return (the `.skip` path leaves it unproduced — an existence-lie). `Consumer`
/// relies on that out to define d0 for a later `jbsr NeedsD0`. Under VERIFIED
/// credit the lie is NOT credited ⇒ d0 undefined at NeedsD0 ⇒ D1b FIRES. MUTATION:
/// reverting D1b to DECLARED credit (crediting the unverified out) suppresses this
/// firing — so it pins the flip's whole point.
#[test]
fn flip_gate_verified_credit_is_load_bearing() {
    let r = analyze(&[
        "module m\n\
         proc Liar (d1: u16) clobbers(d3) out(d0) {\n\
             cmp.w #0, d1\n\
             beq .skip\n\
             move.l d1, d0\n\
             rts\n\
         .skip:\n\
             rts }\n\
         proc NeedsD0 (d0: u16) clobbers() { rts }\n\
         proc Consumer () clobbers(d1/d3) {\n\
             moveq #0, d1\n\
             jbsr Liar\n\
             jbsr NeedsD0\n\
             rts }\n",
    ]);
    assert!(
        input_fires(&r, "Consumer", "NeedsD0", "d0"),
        "Liar's out(d0) is an existence-lie (unproduced on .skip) → verified credit \
         withholds it → d0 undefined at NeedsD0 → D1b fires: {:?}",
        r.input_firings
    );
}
