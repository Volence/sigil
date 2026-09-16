//! The guard census `sigil build --check` prints comes from the evaluator's own
//! records, and a guard that was never decided is never counted as decided.
//!
//! Two families, two records. A comptime `ensure` verdict increments the
//! evaluator's count, drained onto the module beside its deferred asserts at the
//! two lowering sites that drain them (item guards, data initializers), so the
//! count and the `LinkAssert` list share one provenance. A `LinkAssert` counts as
//! decided only when its condition folded, and only a condition counts: the
//! name check each `extern()` call records is not a guard. An `extern()` naming a
//! symbol the link does not define is refused at the reference, so its guard is
//! neither counted passed nor left inapplicable. These gates drive a two-file probe (a
//! guarded module plus an AS equ carrier) through the same lowering, the real
//! `check_link_asserts`, the real drift verdict and the census constructor the
//! check-only resolve uses, with no aeon tree.
//!
//! The decided figure is OBSERVED, read off the link's own
//! [`sigil_link::LinkAssertTally`], not worked out as `conditions - inapplicable`.
//! A gate that only re-checked the observation against that subtraction would be
//! circular, since both count the same population, so two cases here build
//! populations where the two genuinely DISAGREE and pin the observation as the
//! right answer: [`a_condition_that_folded_to_nothing_is_not_counted_decided`]
//! (observed 1, derived 3) and
//! [`an_inapplicable_with_no_unresolvable_condition_is_refused`] (observed 1,
//! derived 0). [`derived_decided`] keeps the superseded arithmetic alive in this
//! file, and nowhere else, so those disagreements can be stated rather than
//! described.
//!
//! ```text
//! cargo test -p sigil-harness --test check_only_census
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_file;
use sigil_frontend_emp::resolve::place_sequential;
use sigil_harness::diag_render::SourceTexts;
use sigil_harness::native::{declared_chain_drift_verdict, GuardCensus};
use sigil_ir::backend::Cpu;
use sigil_ir::{AssertKind, Module, Section, SectionPlacement, SymbolTable};
use sigil_span::{Diagnostic, Level, Span};
use std::path::Path;

const PROBE_PATH: &str = "probe/census.emp";

/// One item-position guard, two guards reached inside a comptime fn called from
/// a data initializer, three `extern()` guards that defer to link.
const PROBE_SRC: &str = "module probe.census\n\
                         const MIRROR_A = 5\n\
                         const MIRROR_B = 9\n\
                         comptime fn checked(x: int) -> int {\n\
                             ensure(x > 0, \"positive\")\n\
                             return x\n\
                         }\n\
                         ensure(MIRROR_A < MIRROR_B, \"ordered\")\n\
                         ensure(extern(\"PROBE_TRUTH_A\") == MIRROR_A, \"MIRROR_A drifted: {MIRROR_A}\")\n\
                         ensure(extern(\"PROBE_TRUTH_B\") == MIRROR_B, \"MIRROR_B drifted: {MIRROR_B}\")\n\
                         ensure(extern(\"PROBE_TRUTH_A\") < 100, \"out of range\")\n\
                         data Cells: [u8; 2] = [checked(1), checked(2)]\n";

/// The same module with the item guard false: a failed verdict is still a verdict.
const PROBE_FAILING_SRC: &str = "module probe.census\n\
                                 const MIRROR_A = 5\n\
                                 const MIRROR_B = 9\n\
                                 comptime fn checked(x: int) -> int {\n\
                                     ensure(x > 0, \"positive\")\n\
                                     return x\n\
                                 }\n\
                                 ensure(MIRROR_A > MIRROR_B, \"reversed\")\n\
                                 data Cells: [u8; 2] = [checked(1), checked(2)]\n";

/// The gated-off-twin probe. `bankid(sym)` lowers to a residual `Sym` leaf with
/// NO `extern()` reference record beside it, so when the link does not define the
/// name the condition poisons WITHOUT a refusal and reaches the drift verdict's
/// inapplicable partition.
///
/// That CLASS is what matters, not this builtin: `winptr()` and an
/// immediate-normalized label leave the same bare leaf, while an undefined
/// `extern()` is refused at its own reference and never gets there. `bankid` is
/// the cheapest representative, and the cross-path check's passing side has to be
/// exercised through one of them or not at all.
const TWIN_PATH: &str = "probe/twin.emp";
const TWIN_SRC: &str = "module probe.twin\n\
                        ensure(bankid(\"PROBE_ABSENT_BANK\") == 0, \"twin bank guard\")\n\
                        data Cells: [u8; 2] = [1, 2]\n";

/// The hijack probe: a guard that FIRES (its condition folds to zero, a real
/// verdict) whose AUTHOR-WRITTEN message happens to carry the exact phrase the
/// drift verdict partitions on. Nothing stops an author writing it.
const HIJACK_PATH: &str = "probe/hijack.emp";
const HIJACK_SRC: &str = "module probe.hijack\n\
                          const MIRROR_A = 5\n\
                          ensure(extern(\"PROBE_TRUTH_A\") == MIRROR_A, \"`PROBE_TRUTH_A` not defined in this link\")\n\
                          data Cells: [u8; 2] = [1, 2]\n";

fn lower_at(path: &str, src: &str) -> (Module, Vec<Diagnostic>, SourceTexts) {
    let mut texts = SourceTexts::new();
    let (file, pdiags) = parse_file(src, texts.add(Path::new(path), src));
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "probe parse: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let (module, ldiags) = lower_module(&file, &opts);
    (module, ldiags, texts)
}

fn lower(src: &str) -> (Module, Vec<Diagnostic>, SourceTexts) {
    lower_at(PROBE_PATH, src)
}

/// Run a lowered module through the real `resolve_layout` and the real TALLIED
/// link-assert check against an AS carrier defining `truths`. Returns the
/// diagnostics and the link's own observation of what each condition did.
fn decide(
    module: &Module,
    truths: &[(&str, &str)],
) -> (Vec<Diagnostic>, sigil_link::LinkAssertTally) {
    let mut sections: Vec<Section> = module.sections.clone();
    place_sequential(&mut sections, 0);
    let mut carriers = sigil_harness::test_support::assemble_equ_pairs(truths);
    for sec in &mut carriers {
        sec.lma = 0x0010_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let (adiags, tally) =
        sigil_link::check_link_asserts_tallied(&resolved, &SymbolTable::new(), &module.link_asserts);
    // The untallied entry point is the same walk with the record dropped, so no
    // caller of either is reading a different set of guards than the other.
    assert_eq!(
        adiags,
        sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &module.link_asserts),
        "the tallied and untallied checks must report the same diagnostics"
    );
    assert!(tally.accounted(), "every condition must land in exactly one bucket: {tally:?}");
    (adiags, tally)
}

/// What the SUPERSEDED derivation would have reported: the condition count minus
/// the inapplicable count. Kept in the gate, and nowhere else, so a test can show
/// a population where it and the observation genuinely disagree.
fn derived_decided(module: &Module, inapplicable: usize) -> usize {
    let conditions =
        module.link_asserts.iter().filter(|a| a.kind == AssertKind::Condition).count();
    conditions - inapplicable
}

/// Lower the probe, link it against a carrier defining `truths`, and return the
/// census the check-only resolve would print, plus the verdict's diagnostics and
/// the link's tally.
fn census(truths: &[(&str, &str)]) -> (GuardCensus, Vec<Diagnostic>, sigil_link::LinkAssertTally) {
    let (module, ldiags, texts) = lower(PROBE_SRC);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "probe lower: {ldiags:?}");
    let (adiags, tally) = decide(&module, truths);
    let inapplicable = declared_chain_drift_verdict(&adiags, &|s: Span| texts.locate(s))
        .unwrap_or_else(|e| panic!("no real drift expected: {e}"));
    let census = GuardCensus::from_verdict(module.comptime_guards, &tally, &inapplicable)
        .unwrap_or_else(|e| panic!("census: {e}"));
    (census, adiags, tally)
}

/// The comptime count is the evaluator's record of verdicts, drained where the
/// deferred asserts are drained: the item guard and the two `checked()` guards the
/// data initializer reaches are counted, the three `extern()` guards are deferred
/// and not counted, so the two families partition the module's six guards.
#[test]
fn comptime_verdicts_are_counted_where_asserts_are_drained() {
    let (module, ldiags, _) = lower(PROBE_SRC);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "{ldiags:?}");
    assert_eq!(module.comptime_guards, 3, "one item guard + two fn guards from the initializer");
    let kinds = |k| module.link_asserts.iter().filter(|a| a.kind == k).count();
    assert_eq!(kinds(AssertKind::Condition), 3, "every extern() guard defers");
    assert_eq!(kinds(AssertKind::ExternDefined), 3, "one name check per extern() call site");
}

/// A failed comptime guard is a verdict too: the count does not drop to two
/// because one of the three said no, and the failure is on the diagnostics.
#[test]
fn a_failed_comptime_guard_is_still_a_verdict() {
    let (module, ldiags, texts) = lower(PROBE_FAILING_SRC);
    let errs: Vec<&Diagnostic> = ldiags.iter().filter(|d| d.level == Level::Error).collect();
    assert_eq!(errs.len(), 1, "{ldiags:?}");
    assert_eq!(errs[0].message, "reversed");
    assert_eq!(texts.locate(errs[0].primary).as_deref(), Some("probe/census.emp:8:1"));
    assert_eq!(module.comptime_guards, 3);
    assert!(module.link_asserts.is_empty());
}

/// With both truths defined every `LinkAssert` folds: three decided, none
/// inapplicable, and the drift verdict has nothing to say.
#[test]
fn every_defined_extern_guard_is_decided() {
    let (census, adiags, tally) = census(&[("PROBE_TRUTH_A", "5"), ("PROBE_TRUTH_B", "9")]);
    assert!(adiags.is_empty(), "{adiags:?}");
    // The census's decided figure IS the tally's, watched at the fold.
    assert_eq!(tally.conditions, 3, "{tally:?}");
    assert_eq!(tally.decided, 3, "{tally:?}");
    assert_eq!(tally.decided_fail, 0, "three silent passes: {tally:?}");
    assert_eq!(tally.undecided, 0, "{tally:?}");
    assert!(tally.unresolvable_spans.is_empty(), "{tally:?}");
    assert_eq!(
        census,
        GuardCensus { comptime_guards: 3, link_asserts_decided: 3, link_asserts_inapplicable: 0 }
    );
}

/// A guard that FAILS is decided: the condition folded to zero, which is a
/// verdict, so it is inside `decided` and not beside it. A census that counted
/// only the passes would under-report the work the link did.
#[test]
fn a_failing_link_guard_is_decided_not_missing() {
    let (module, _, texts) = lower(PROBE_SRC);
    let (adiags, tally) = decide(&module, &[("PROBE_TRUTH_A", "6"), ("PROBE_TRUTH_B", "9")]);
    assert_eq!(tally.conditions, 3, "{tally:?}");
    assert_eq!(tally.decided, 3, "the drifted guard decided `no`: {tally:?}");
    assert_eq!(tally.decided_fail, 1, "{tally:?}");
    assert_eq!(tally.undecided, 0, "{tally:?}");
    // The resolve still fails on it; the point is that it was COUNTED as a verdict.
    assert!(declared_chain_drift_verdict(&adiags, &|s: Span| texts.locate(s)).is_err());
}

/// THE anti-vacuity case, and the whole reason the figure is observed rather than
/// subtracted. `PROBE_TRUTH_A` is absent, so each `extern("PROBE_TRUTH_A")` is
/// refused at its own reference and the two guards that read it fold to NOTHING:
/// they are skipped without a verdict and without a diagnostic of their own.
///
/// The two figures genuinely disagree on this population. The observation says
/// ONE guard decided (the `PROBE_TRUTH_B` one, which really did fold). The
/// superseded derivation, `conditions - inapplicable`, says THREE, because
/// neither skipped guard is inapplicable and subtraction has no way to notice a
/// guard that produced nothing at all. The gate would go red on a revert.
///
/// What keeps this off a shipping build today is a SEPARATE fact, asserted below:
/// the refusal makes the drift verdict fail, so no census is printed from this
/// population. That shield is why the old figure was not a live wrong number, and
/// it is exactly what an observed count no longer has to depend on.
#[test]
fn a_condition_that_folded_to_nothing_is_not_counted_decided() {
    let (module, _, texts) = lower(PROBE_SRC);
    let (adiags, tally) = decide(&module, &[("PROBE_TRUTH_B", "9")]);
    assert_eq!(tally.conditions, 3, "{tally:?}");
    assert_eq!(tally.decided, 1, "only the PROBE_TRUTH_B guard folded: {tally:?}");
    assert_eq!(tally.undecided, 2, "both PROBE_TRUTH_A guards folded to nothing: {tally:?}");
    assert!(
        tally.unresolvable_spans.is_empty(),
        "a skipped guard is not an inapplicable twin either: {tally:?}"
    );

    // Neither skipped guard is inapplicable, so the two figures differ by two.
    let inapplicable: Vec<&Diagnostic> = Vec::new();
    assert_eq!(derived_decided(&module, inapplicable.len()), 3, "the superseded derivation");
    let census = GuardCensus::from_verdict(module.comptime_guards, &tally, &inapplicable).unwrap();
    assert_eq!(census.link_asserts_decided, 1, "the observation");
    assert_ne!(
        census.link_asserts_decided,
        derived_decided(&module, inapplicable.len()),
        "this probe exists to make the two figures disagree; if they agree the gate is vacuous"
    );

    // The structural shield, stated as its own fact rather than assumed.
    let err = declared_chain_drift_verdict(&adiags, &|s: Span| texts.locate(s)).unwrap_err();
    assert!(err.starts_with("extern() names a symbol no module in this link defines:"), "{err}");
}

/// The passing side of the census's cross-path check, on a shape that actually
/// reaches the inapplicable partition. `bankid("PROBE_ABSENT_BANK")`
/// leaves a `Sym` leaf with no `extern()` reference record, so the condition
/// poisons with no refusal: the link RECORDS its span unresolvable, the verdict
/// partitions the same guard as an inapplicable gated-off twin, and the two
/// independently produced populations are matched one-to-one.
#[test]
fn a_gated_off_twin_is_recorded_unresolvable_and_matched_to_the_partition() {
    let (module, ldiags, texts) = lower_at(TWIN_PATH, TWIN_SRC);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "twin lower: {ldiags:?}");
    assert_eq!(
        module.link_asserts.iter().filter(|a| a.kind == AssertKind::ExternDefined).count(),
        0,
        "bankid records no extern() reference, which is what makes this shape reachable"
    );
    let (adiags, tally) = decide(&module, &[]);
    assert_eq!(tally.conditions, 1, "{tally:?}");
    assert_eq!(tally.decided, 0, "an unresolvable twin guard decides nothing: {tally:?}");
    assert_eq!(tally.undecided, 1, "{tally:?}");
    assert_eq!(tally.unresolvable_spans.len(), 1, "{tally:?}");

    let inapplicable = declared_chain_drift_verdict(&adiags, &|s: Span| texts.locate(s))
        .unwrap_or_else(|e| panic!("a gated-off twin is not real drift: {e}"));
    assert_eq!(inapplicable.len(), 1, "{inapplicable:?}");
    assert_eq!(inapplicable[0].primary, tally.unresolvable_spans[0], "same guard, both paths");
    let census = GuardCensus::from_verdict(module.comptime_guards, &tally, &inapplicable)
        .unwrap_or_else(|e| panic!("the cross-path check must PASS here: {e}"));
    assert_eq!(census.link_asserts_decided, 0);
    assert_eq!(census.link_asserts_inapplicable, 1);
}

/// The cross-path check going red on a genuine disagreement. The drift verdict
/// partitions on the TEXT "not defined in this link", which an author is free to
/// write into a guard message. Here a guard FIRES (folds to zero, a real verdict,
/// real drift) and its own message carries that phrase, so the verdict hands it
/// back as an inapplicable twin although the link recorded no unresolvable
/// condition anywhere.
///
/// The superseded derivation would have reported ZERO decided guards on a build
/// where exactly one guard decided, and said nothing about the fired guard being
/// reclassified. The observed census refuses to report at all, naming the
/// diagnostic. This is the only shape found in which the two code paths can
/// disagree about what a condition is.
#[test]
fn an_inapplicable_with_no_unresolvable_condition_is_refused() {
    let (module, ldiags, texts) = lower_at(HIJACK_PATH, HIJACK_SRC);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "hijack lower: {ldiags:?}");
    let (adiags, tally) = decide(&module, &[("PROBE_TRUTH_A", "6")]);
    assert_eq!(tally.conditions, 1, "{tally:?}");
    assert_eq!(tally.decided, 1, "the guard really did fold, to zero: {tally:?}");
    assert_eq!(tally.decided_fail, 1, "{tally:?}");
    assert!(tally.unresolvable_spans.is_empty(), "nothing was unresolvable: {tally:?}");

    // The verdict mis-partitions the fired guard on its wording alone.
    let inapplicable = declared_chain_drift_verdict(&adiags, &|s: Span| texts.locate(s))
        .unwrap_or_else(|e| panic!("the phrase steers it away from the real-drift arm: {e}"));
    assert_eq!(inapplicable.len(), 1, "{inapplicable:?}");
    assert_eq!(derived_decided(&module, inapplicable.len()), 0, "the superseded derivation");

    let err = GuardCensus::from_verdict(module.comptime_guards, &tally, &inapplicable)
        .expect_err("a census must not be reported from two populations that disagree");
    assert!(err.contains("partitioned as an INAPPLICABLE guard"), "{err}");
    assert!(err.contains("PROBE_TRUTH_A"), "the offending diagnostic is named: {err}");
}

/// The control: a guard whose extern this link does not define is NOT decided, and
/// it is not passed either. With `PROBE_TRUTH_A` absent, each `extern("PROBE_TRUTH_A")`
/// is refused at its own located line, the two guards that read it add no second
/// diagnostic, and the verdict fails instead of producing a census.
#[test]
fn an_undefined_extern_is_refused_at_each_reference() {
    let (module, _, texts) = lower(PROBE_SRC);
    let mut sections: Vec<Section> = module.sections;
    place_sequential(&mut sections, 0);
    let mut carriers = sigil_harness::test_support::assemble_equ_pairs(&[("PROBE_TRUTH_B", "9")]);
    for sec in &mut carriers {
        sec.lma = 0x0010_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let adiags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &module.link_asserts);
    assert_eq!(adiags.len(), 2, "{adiags:?}");
    assert!(
        adiags.iter().all(|d| d.message.starts_with(sigil_link::EXTERN_UNKNOWN_ID)
            && d.message.contains("`PROBE_TRUTH_A`")),
        "{adiags:?}"
    );
    let err = declared_chain_drift_verdict(&adiags, &|s: Span| texts.locate(s)).unwrap_err();
    assert_eq!(
        err.lines().next(),
        Some("extern() names a symbol no module in this link defines: 2 error(s):"),
        "{err}"
    );
    assert!(err.contains("probe/census.emp:9:8: [Error] [extern.unknown]"), "{err}");
    assert!(err.contains("probe/census.emp:11:8: [Error] [extern.unknown]"), "{err}");
}

/// A drifted truth is real drift: the verdict fails with the guard's own located
/// line, exactly as the full build renders it, and no census is produced.
#[test]
fn a_drifted_extern_guard_fails_the_verdict_with_its_line() {
    let (module, _, texts) = lower(PROBE_SRC);
    let mut sections: Vec<Section> = module.sections;
    place_sequential(&mut sections, 0);
    let mut carriers = sigil_harness::test_support::assemble_equ_pairs(&[("PROBE_TRUTH_A", "6"), ("PROBE_TRUTH_B", "9")]);
    for sec in &mut carriers {
        sec.lma = 0x0010_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let adiags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &module.link_asserts);
    let err = declared_chain_drift_verdict(&adiags, &|s: Span| texts.locate(s)).unwrap_err();
    assert_eq!(err.lines().next(), Some("declared-chain drift guard FIRED: 1 error(s):"), "{err}");
    assert!(err.contains("probe/census.emp:9:1: [Error] MIRROR_A drifted: 5"), "{err}");
}
