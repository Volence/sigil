//! Link-assert failures reach the reader as located diagnostics, all of them.
//!
//! An `ensure` whose condition contains `extern(..)` is a `LinkAssert`: it is
//! decided by `check_link_asserts` after `resolve_layout`, long after the
//! frontend's diagnostics have been rendered. Two things used to go wrong on the
//! way from that verdict to the reader, both measured from aeon's build log:
//!
//!   * the declared-chain build reported `FIRED: N error(s); first Some(Diagnostic
//!     { .. })`, a `{:?}` of one diagnostic with no `[Error]` token, so a lane that
//!     counts `[Error]` lines counted zero for any number of failing guards;
//!   * the sound emitters' co-link guards (mt_bank, sfx, dac head, ..) returned a
//!     `{:?}` of the whole list, and the build driver unwrapped that into a panic,
//!     exit 101, indistinguishable from a crash.
//!
//! These gates drive a two-file probe (a guarded `.emp` module plus an AS equ
//! carrier that supplies the `extern()` truths) through the real
//! `check_link_asserts` and the real verdict functions, with no aeon tree.
//!
//! ```text
//! cargo test -p sigil-harness --test link_assert_reporting
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_file;
use sigil_frontend_emp::resolve::place_sequential;
use sigil_harness::diag_render::{link_assert_failure, SourceTexts};
use sigil_harness::native::declared_chain_drift_verdict;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_span::{Diagnostic, Level, Span};
use std::path::Path;

/// The guarded module: two mirrors of an authority it can only reach through
/// `extern()`, plus one guard that holds whatever the authority says.
const GUARDED_PATH: &str = "probe/guarded.emp";
const GUARDED_SRC: &str = "module probe.guarded\n\
                           const MIRROR_A = 5\n\
                           const MIRROR_B = 9\n\
                           ensure(extern(\"PROBE_TRUTH_A\") == MIRROR_A, \"MIRROR_A drifted from the authority: {MIRROR_A}\")\n\
                           ensure(extern(\"PROBE_TRUTH_B\") == MIRROR_B, \"MIRROR_B drifted from the authority: {MIRROR_B}\")\n\
                           ensure(extern(\"PROBE_TRUTH_A\") < 100, \"the authority is out of range\")\n\
                           data Probe_Cells: [u8; 2] = [1, 2]\n";

/// Lower the guarded module, link it against an AS carrier that defines the
/// named truths, and return the link-assert diagnostics plus the locator that
/// explains their spans.
fn probe(truths: &[(&str, &str)]) -> (Vec<Diagnostic>, SourceTexts) {
    let mut texts = SourceTexts::new();
    let (file, pdiags) = parse_file(GUARDED_SRC, texts.add(Path::new(GUARDED_PATH), GUARDED_SRC));
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "probe parse: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "probe lower: {ldiags:?}");
    let guards = module.link_asserts.iter().filter(|a| a.kind == sigil_ir::AssertKind::Condition).count();
    assert_eq!(guards, 3, "every extern() guard defers to link");

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
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &module.link_asserts);
    (diags, texts)
}

/// Both mirrors drifted: the emitter-side verdict names BOTH, each at its own
/// `path:line:col`, each with the `[Error]` token, and never a `{:?}` of a
/// diagnostic.
#[test]
fn emitter_verdict_renders_every_failing_guard_located() {
    let (diags, texts) = probe(&[("PROBE_TRUTH_A", "6"), ("PROBE_TRUTH_B", "10")]);
    assert_eq!(diags.len(), 2, "two of three guards fail: {diags:?}");
    let err = link_assert_failure("probe guards fired", &diags, &|s: Span| texts.locate(s)).unwrap_err();
    let lines: Vec<&str> = err.lines().collect();
    assert_eq!(lines[0], "probe guards fired: 2 error(s):", "{err}");
    assert_eq!(lines[1], "  probe/guarded.emp:4:1: [Error] MIRROR_A drifted from the authority: 5", "{err}");
    assert_eq!(lines[2], "  probe/guarded.emp:5:1: [Error] MIRROR_B drifted from the authority: 9", "{err}");
    assert_eq!(lines.len(), 3, "{err}");
    assert!(!err.contains("Diagnostic {") && !err.contains("first"), "{err}");
}

/// The declared-chain verdict is the SAME rendering under its own header: the
/// count aeon's link rows read, then one `[Error]` line per failing guard.
#[test]
fn declared_chain_verdict_renders_every_failing_guard_located() {
    let (diags, texts) = probe(&[("PROBE_TRUTH_A", "6"), ("PROBE_TRUTH_B", "10")]);
    let err = declared_chain_drift_verdict(&diags, &|s: Span| texts.locate(s)).unwrap_err();
    let lines: Vec<&str> = err.lines().collect();
    assert_eq!(lines[0], "declared-chain drift guard FIRED: 2 error(s):", "{err}");
    assert_eq!(err.matches("[Error]").count(), 2, "{err}");
    assert!(err.contains("probe/guarded.emp:4:1: [Error] MIRROR_A drifted"), "{err}");
    assert!(err.contains("probe/guarded.emp:5:1: [Error] MIRROR_B drifted"), "{err}");
    assert!(!err.contains("Diagnostic {") && !err.contains("first"), "{err}");
}

/// The control: with the authority agreeing, no guard fails, both verdicts pass,
/// and the emitter verdict renders nothing.
#[test]
fn agreeing_authority_passes_both_verdicts() {
    let (diags, texts) = probe(&[("PROBE_TRUTH_A", "5"), ("PROBE_TRUTH_B", "9")]);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(link_assert_failure("probe guards fired", &diags, &|s: Span| texts.locate(s)), Ok(()));
    let inapplicable = declared_chain_drift_verdict(&diags, &|s: Span| texts.locate(s)).unwrap();
    assert!(inapplicable.is_empty());
}

/// A guard whose `extern()` names a symbol this link does not define is not left
/// inapplicable: the reference itself is refused at its own located line, the
/// guards that read it add no second report, and the verdict fails. A real drift
/// beside defined names still fails as drift.
#[test]
fn declared_chain_verdict_refuses_an_undefined_extern_and_still_reports_real_drift() {
    let (diags, texts) = probe(&[("PROBE_TRUTH_B", "9")]);
    // PROBE_TRUTH_A is undefined: both references to it are refused, guard 2 holds.
    assert_eq!(diags.len(), 2, "{diags:?}");
    let err = declared_chain_drift_verdict(&diags, &|s: Span| texts.locate(s)).unwrap_err();
    assert_eq!(
        err.lines().next(),
        Some("extern() names a symbol no module in this link defines: 2 error(s):"),
        "{err}"
    );
    assert!(err.contains("probe/guarded.emp:4:8: [Error] [extern.unknown]"), "{err}");
    assert!(err.contains("probe/guarded.emp:6:8: [Error] [extern.unknown]"), "{err}");

    let (diags, texts) = probe(&[("PROBE_TRUTH_A", "5"), ("PROBE_TRUTH_B", "10")]);
    let err = declared_chain_drift_verdict(&diags, &|s: Span| texts.locate(s)).unwrap_err();
    assert_eq!(err.matches("[Error]").count(), 1, "{err}");
    assert!(err.contains("probe/guarded.emp:5:1: [Error] MIRROR_B drifted"), "{err}");
}

/// The build driver's emitter step returns its failure instead of panicking. An
/// absent tree is the failure every environment can produce; the point is the
/// channel (`Err`, not exit 101), and that the refusal creates nothing.
#[test]
fn emit_generated_returns_its_failure_and_creates_nothing() {
    let absent = std::env::temp_dir().join(format!("sigil-linkassert-absent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&absent);
    let outcome = std::panic::catch_unwind(|| sigil_harness::native::emit_generated(&absent));
    let created = absent.exists();
    let _ = std::fs::remove_dir_all(&absent);
    let result = outcome.expect("emit_generated must not panic");
    let err = result.expect_err("an absent tree is a refusal");
    assert!(err.contains(&absent.display().to_string()), "the refusal names the tree: {err}");
    assert!(!created, "the refusal created {}", absent.display());
}
