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
//! sub-machine ([`is_opcode_dispatch_proc`]) is the design §4 face-4 / OQ-4 boundary
//! — its transitive clobbers depend on the un-traversable computed edge — so it is
//! reported SEPARATELY, never as an in-scope firing.
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

use sigil_harness::seam1::{
    in_scope_firings, is_opcode_dispatch_proc, z80_clobbers_report, z80_clobbers_report_doctored,
};
use sigil_harness::test_support::reference_tree;
use std::path::PathBuf;

/// The reference tree the clobbers closure reads: the five resident modules plus
/// the constant authorities their `-D` defines resolve against. `None` skips.
fn sound_tree() -> Option<PathBuf> {
    reference_tree(&[
        "engine/sound/z80_sound_driver.emp",
        "engine/sound/sound_sequencer.emp",
        "engine/sound/sound_sfx.emp",
        "engine/sound/sound_fm.emp",
        "engine/sound/sound_psg.emp",
        "engine/sound/sound_constants.emp",
        "engine/system/constants.emp",
    ])
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
        let in_scope = in_scope_firings(&report);
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
    let fired: Vec<_> = in_scope_firings(&report)
        .into_iter()
        .filter(|f| f.proc == "Sfx_Frame")
        .collect();
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
        !in_scope_firings(&honest).iter().any(|f| f.proc == "Sfx_Steal"),
        "Sfx_Steal must be honest un-doctored"
    );
    // Doctored to omit iy: fires iy.
    let doctor = [("Sfx_Steal", segs(&["af", "bc", "de", "hl"]))];
    let report = z80_clobbers_report_doctored(&aeon, false, &doctor);
    assert!(
        in_scope_firings(&report)
            .iter()
            .any(|f| f.proc == "Sfx_Steal" && f.reg.as_deref() == Some("iy")),
        "the injected Sfx_Steal iy under-claim must fire; got {:?}",
        report.firings.iter().map(|f| format!("{}/{:?}", f.proc, f.reg)).collect::<Vec<_>>()
    );
}

// ===========================================================================
// SCOPE — the excluded dispatch sub-machine is genuinely present (not a vacuous
// filter hiding a broken analysis).
// ===========================================================================

/// The scope filter is NON-VACUOUS: the computed-dispatch sub-machine DOES fire
/// (its `Seq_Op_*`/`Seq_Hook*`/`Seq_ContinueFetch` procs under-declare the loop-
/// threaded scratch), and every one of those firings is correctly classified as a
/// dispatch proc — so `in_scope_firings` is filtering REAL firings, not masking an
/// empty analysis.
#[test]
fn dispatch_submachine_is_excluded_but_present() {
    let Some(aeon) = sound_tree() else { return };
    let report = z80_clobbers_report(&aeon, false);
    let excluded: Vec<_> =
        report.firings.iter().filter(|f| is_opcode_dispatch_proc(&f.proc)).collect();
    assert!(
        !excluded.is_empty(),
        "the analysis is vacuous if the dispatch sub-machine fires nothing"
    );
    // Every excluded firing is a dispatch proc, and every total firing is EITHER
    // in-scope (0, per the honest corpus) or a dispatch proc — the partition is total.
    assert_eq!(
        excluded.len(),
        report.firings.len(),
        "on the honest corpus every firing is a dispatch proc (in-scope must be 0); \
         non-dispatch firings: {:?}",
        in_scope_firings(&report)
            .iter()
            .map(|f| format!("{}/{:?}", f.proc, f.reg))
            .collect::<Vec<_>>()
    );
}
