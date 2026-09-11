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

fn lower(src: &str) -> (Module, Vec<Diagnostic>, SourceTexts) {
    let mut texts = SourceTexts::new();
    let (file, pdiags) = parse_file(src, texts.add(Path::new(PROBE_PATH), src));
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "probe parse: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let (module, ldiags) = lower_module(&file, &opts);
    (module, ldiags, texts)
}

/// Lower the probe, link it against a carrier defining `truths`, and return the
/// census the check-only resolve would print, plus the verdict's diagnostics.
fn census(truths: &[(&str, &str)]) -> (GuardCensus, Vec<Diagnostic>) {
    let (module, ldiags, texts) = lower(PROBE_SRC);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "probe lower: {ldiags:?}");

    let mut sections: Vec<Section> = module.sections;
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
    let adiags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &module.link_asserts);
    let inapplicable = declared_chain_drift_verdict(&adiags, &|s: Span| texts.locate(s))
        .unwrap_or_else(|e| panic!("no real drift expected: {e}"));
    let census = GuardCensus::from_verdict(module.comptime_guards, &module.link_asserts, &inapplicable);
    (census, adiags)
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
    let (census, adiags) = census(&[("PROBE_TRUTH_A", "5"), ("PROBE_TRUTH_B", "9")]);
    assert!(adiags.is_empty(), "{adiags:?}");
    assert_eq!(
        census,
        GuardCensus { comptime_guards: 3, link_asserts_decided: 3, link_asserts_inapplicable: 0 }
    );
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
