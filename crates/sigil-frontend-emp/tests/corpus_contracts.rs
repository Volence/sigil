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
