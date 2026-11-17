//! `[call.clobbers-incomplete]` — the TRANSITIVE-clobbers-completeness diagnostic
//! over the linked resident sound blob (seam-1 design §4, the t37 demand).
//!
//! Each proc's declared `clobbers` must be a SUPERSET of its transitive effect
//! (local writes ∪ reachable-callee clobbers − verified preserves). Z80 has NO
//! per-proc clobber lint (68k-only), so before the native link nothing verified
//! this across `call`s (the t37 checker-invisibility proof). The native link makes
//! it computable — every callee body is present in ONE module set, so the
//! reachable-callee union is finite and in-scope.
//!
//! SCOPE: the direct-call closure. The `ex(sp),hl; ret` computed opcode-dispatch
//! sub-machine is the design §4 face-4 / OQ-4 boundary — its transitive clobbers
//! depend on the un-traversable computed edge — so it is reported SEPARATELY, never
//! as an in-scope firing. Since A5 the boundary is DECLARED in source, each such
//! proc carrying `@allow("clobbers.computed-dispatch", "<reason>")`, and the
//! partition is by that annotation, never by name. The build gate runs the same
//! closure over the whole corpus (`z80_clobber_gate.rs`).
//!
//! REFERENCE-DEPENDENT: the five resident sound `.emp` modules live in the
//! sibling `aeon` tree (`AEON_DIR`, or `EMPYREAN_SUITE_ROOT`).
//! Absent, every test here SKIPS green — unless `SIGIL_STRICT_GATE=1` makes a
//! missing reference a hard failure, so the pre-merge run cannot skip the
//! diagnostic.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test z80_clobbers_incomplete
//! ```

use sigil_harness::seam1::{z80_clobbers_report, z80_clobbers_report_doctored};
use sigil_harness::test_support::reference_tree;
use std::path::{Path, PathBuf};

/// The five resident modules the seam-1 closure walks (the report's input set).
const RESIDENT: [&str; 5] = [
    "engine/sound/z80_sound_driver.emp",
    "engine/sound/sound_sequencer.emp",
    "engine/sound/sound_sfx.emp",
    "engine/sound/sound_fm.emp",
    "engine/sound/sound_psg.emp",
];

/// The reference tree the clobbers closure reads: the five resident modules plus
/// the constant authorities their `-D` defines resolve against. `None` skips.
fn sound_tree() -> Option<PathBuf> {
    let mut rels = RESIDENT.to_vec();
    rels.extend(["engine/sound/sound_constants.emp", "engine/system/constants.emp"]);
    reference_tree(&rels)
}

/// The procs of `rels` that carry `@allow("clobbers.computed-dispatch", …)`, read
/// from the SOURCE TEXT (not from the analysis under test): each attribute line
/// names the first `proc NAME` declaration after it, past blank, comment and other
/// attribute lines. Sorted.
fn annotated_in_source(aeon: &Path, rels: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for rel in rels {
        let src = std::fs::read_to_string(aeon.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        let lines: Vec<&str> = src.lines().map(str::trim).collect();
        for (i, l) in lines.iter().enumerate() {
            if !l.starts_with("@allow(\"clobbers.computed-dispatch\"") {
                continue;
            }
            let decl = lines[i + 1..]
                .iter()
                .find(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with('@'))
                .unwrap_or_else(|| panic!("{rel}:{}: the attribute precedes no declaration", i + 1));
            let name = decl
                .trim_start_matches("pub ")
                .strip_prefix("proc ")
                .and_then(|r| r.split(|c: char| !(c.is_alphanumeric() || c == '_')).next())
                .unwrap_or_else(|| panic!("{rel}:{}: the attribute's declaration is not a proc: {decl}", i + 1));
            out.push(name.to_string());
        }
    }
    out.sort();
    out
}

/// Reglist segments from a set of bare Z80 register names.
fn segs(names: &[&str]) -> Vec<(String, Option<String>)> {
    names.iter().map(|n| (n.to_string(), None)).collect()
}

// ===========================================================================
// GREEN — the honest corpus fires 0 in scope (both shapes).
// ===========================================================================

/// The corpus is complete: no in-scope proc under-claims its transitive clobbers,
/// in EITHER shape (the contract is shape-invariant; the debug shape's extra
/// `Seq_Trace` calls must not open a new leak). The dispatch sub-machine is out of
/// scope (reported separately, asserted present in `dispatch_submachine_is_excluded`).
#[test]
fn honest_corpus_no_in_scope_firings() {
    let Some(aeon) = sound_tree() else { return };
    for debug in [false, true] {
        let report = z80_clobbers_report(&aeon, debug);
        let in_scope = &report.firings;
        assert!(
            in_scope.is_empty(),
            "shape debug={debug}: {} in-scope [call.clobbers-incomplete] firings: {:?}",
            in_scope.len(),
            in_scope
                .iter()
                .map(|f| format!("{}/{:?}", f.proc, f.reg))
                .collect::<Vec<_>>()
        );
    }
}

/// The closure is COMPLETE over the corpus: no proc-body instruction was DROPPED
/// (unresolved mnemonic/operand), and no resident direct callee escapes the blob
/// (OQ-4: every resident code-call is intra-blob — the banked side is data-read
/// only, so the fixpoint closes against seam-1 alone).
#[test]
fn closure_is_complete_no_drops_no_unresolved() {
    let Some(aeon) = sound_tree() else { return };
    for debug in [false, true] {
        let report = z80_clobbers_report(&aeon, debug);
        assert_eq!(report.dropped, 0, "shape debug={debug}: {} dropped instructions", report.dropped);
        assert!(
            report.unresolved_callees.is_empty(),
            "shape debug={debug}: resident code-calls escape the blob (OQ-4 breach): {:?}",
            report.unresolved_callees
        );
    }
}

// ===========================================================================
// RED — the t37 Sfx_Frame iy under-claim (reverted to the pre-t37 header) FIRES.
// ===========================================================================

/// The demanded RED fixture: revert `Sfx_Frame`'s clobbers to the pre-t37 header
/// `clobbers(af,bc,de,hl,ix)` — OMITTING iy, which its callees (Sfx_DrainQueue /
/// Sfx_DuckRamp / Sfx_Restore) destroy. `[call.clobbers-incomplete]` MUST fire on
/// `Sfx_Frame`/iy (transitively). This is the leak that was checker-invisible before
/// the native link (t37 §3.1). Sfx_Frame is IN scope (not a dispatch proc).
#[test]
fn red_fixture_sfx_frame_iy_underclaim_fires() {
    let Some(aeon) = sound_tree() else { return };
    let doctor = [("Sfx_Frame", segs(&["af", "bc", "de", "hl", "ix"]))];
    let report = z80_clobbers_report_doctored(&aeon, false, &doctor);
    let fired: Vec<_> = report.firings.iter().filter(|f| f.proc == "Sfx_Frame").collect();
    assert!(
        fired.iter().any(|f| f.reg.as_deref() == Some("iy") && f.transitive),
        "the reverted Sfx_Frame iy under-claim must fire transitively; got {:?}",
        report.firings.iter().map(|f| format!("{}/{:?}", f.proc, f.reg)).collect::<Vec<_>>()
    );
    // And the register is iy specifically — no other register spuriously fires on
    // Sfx_Frame (its honest header adds ONLY iy over the reverted one).
    assert!(
        fired.iter().all(|f| f.reg.as_deref() == Some("iy")),
        "only iy should fire on the reverted Sfx_Frame; got {:?}",
        fired.iter().map(|f| f.reg.clone()).collect::<Vec<_>>()
    );
}

/// Non-vacuity (t24), second axis: an injected under-claim on a DIFFERENT real proc
/// fires, and the SAME proc un-doctored does not — so the check is sensitive to the
/// individual contract, not vacuously green. `Sfx_Steal` honestly clobbers iy
/// (`clobbers(af,bc,de,hl,iy)`); doctoring it to omit iy fires iy.
#[test]
fn non_vacuity_injected_underclaim_on_sfx_steal() {
    let Some(aeon) = sound_tree() else { return };
    // Honest: no in-scope firing on Sfx_Steal.
    let honest = z80_clobbers_report(&aeon, false);
    assert!(
        !honest.firings.iter().any(|f| f.proc == "Sfx_Steal"),
        "Sfx_Steal must be honest un-doctored"
    );
    // Doctored to omit iy: fires iy.
    let doctor = [("Sfx_Steal", segs(&["af", "bc", "de", "hl"]))];
    let report = z80_clobbers_report_doctored(&aeon, false, &doctor);
    assert!(
        report.firings.iter().any(|f| f.proc == "Sfx_Steal" && f.reg.as_deref() == Some("iy")),
        "the injected Sfx_Steal iy under-claim must fire; got {:?}",
        report.firings.iter().map(|f| format!("{}/{:?}", f.proc, f.reg)).collect::<Vec<_>>()
    );
}

// ===========================================================================
// SCOPE — the excluded dispatch sub-machine is genuinely present (not a vacuous
// filter hiding a broken analysis).
// ===========================================================================

/// The scope filter is NON-VACUOUS and DECLARED: the computed-dispatch
/// sub-machine DOES fire (its handlers under-declare the loop-threaded scratch),
/// every one of those firings is on an annotated proc, and the excluded set is
/// exactly the procs annotated in source, scanned from the text rather than
/// listed here. Both shapes.
#[test]
fn dispatch_submachine_is_excluded_by_annotation_but_present() {
    let Some(aeon) = sound_tree() else { return };
    let annotated = annotated_in_source(&aeon, &RESIDENT);
    assert!(
        !annotated.is_empty(),
        "no `@allow(\"clobbers.computed-dispatch\")` in the resident modules, so the \
         partition below would be checked against nothing"
    );
    for debug in [false, true] {
        let report = z80_clobbers_report(&aeon, debug);
        assert_eq!(
            report.excluded, annotated,
            "shape debug={debug}: the excluded set must be exactly the annotated procs"
        );
        assert!(
            !report.excluded_firings.is_empty(),
            "shape debug={debug}: the analysis is vacuous if the dispatch sub-machine fires nothing"
        );
        assert!(
            report.excluded_firings.iter().all(|f| annotated.contains(&f.proc)),
            "shape debug={debug}: an excluded firing on an unannotated proc: {:?}",
            report.excluded_firings.iter().map(|f| &f.proc).collect::<Vec<_>>()
        );
        assert!(
            report.firings.iter().all(|f| !annotated.contains(&f.proc)),
            "shape debug={debug}: an in-scope firing on an annotated proc"
        );
    }
}
