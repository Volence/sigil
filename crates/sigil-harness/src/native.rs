//! What is left of the profile-driven driver (G0b Task 11).
//!
//! The profile-driven driver (`GameProfile`, the seven profile constructors, the frozen
//! size tables, the chained build and its walk) is deleted: every game builds through the
//! game-as-data driver, `sigil_build::driver::build`, from its descriptions
//! (`sigil build --project <game> --shape <row>`). What stays here is not a fact about any
//! game:
//!
//!   * the sound-artifact emitters ([`emit_generated_in`], which the driver's sound seam
//!     calls with the build's own generated root, and [`emit_generated`] /
//!     [`ensure_generated`], which provision a reference tree for a standalone port gate);
//!   * the fold check ([`check_sound_fold`]) the sound seam runs over the resolved layout;
//!   * `native::` re-exports of the game-independent driver code `sigil-build` holds
//!     (G0b Task 4), each one a path some caller still uses.

use std::path::Path;

use sigil_ir::Section;

use crate::{seam1, seam2};

// G0b Task 4: the game-independent driver code moved, unchanged, to `sigil-build`.
// Re-exported here for the callers that still use the `native::` path (a re-export
// nothing uses is deleted, not kept).
pub use sigil_build::appendix::{
    append_deb2_appendix, check_error_handler_is_last, convsym_resolve, AppendixFacts, DEB2_MAGIC,
};
pub use sigil_build::diag::{collect_warnings, BuildWarning, ChainSources};
pub use sigil_build::emit::{assembled_anchor_crc, crc32};
pub use sigil_build::guards::{check_budgets, declared_chain_drift_verdict, validate_placement, GuardCensus};
use sigil_build::place::head_label;

/// The tree directory [`emit_generated`] provisions (the old mount, written only for
/// the step-0 reference tree's standalone port gates and aeon's `build.sh` preflight;
/// no build reads it: G0c removes it with the seams).
pub const TREE_SOUND_MOUNT: &str = "engine/sound/generated";

/// Emit every generated sound artifact into the tree's own generated mount,
/// `<aeon>/engine/sound/generated` ([`TREE_SOUND_MOUNT`]). The
/// sound stack is native; the `.bin`s are how those native bytes enter the ROM, through
/// the `embed`s aeon's sound modules carry.
///
/// NO BUILD CALLS THIS. A build writes into its own generated root
/// ([`emit_generated_in`] from the driver's sound seam,
/// [`crate::driver_seams::HarnessSeams`]), never into the tree it builds (G0b Task 6). This is the
/// provisioning step for a STANDALONE port gate that compiles one sound module against a
/// reference tree with no build around it, so its `embed`s find the files in the tree.
///
/// It is therefore a WRITE into the reference tree. It refuses up front when that tree
/// is not there, or was not named ([`seam2::require_reference_tree_for`]), rather than
/// creating the path and failing afterwards: a manufactured `$AEON_DIR` root turns every
/// root-probing skip guard in the suite into a run against an empty tree, which makes an
/// absent-tree run behave differently the second time than the first. Each emitter
/// re-checks the same precondition, so the `emit_sound_blob` binary and direct callers
/// get the refusal too.
///
/// A failure is the caller's `Err`, never a panic: an emitter's guard firing (a
/// `.emp` `ensure(extern(..))` co-residency or drift guard) is a diagnostic about
/// the source, and it reaches the reader as one, on the same channel and with the
/// same exit status (1) every other build error uses. A panic would exit 101,
/// which is a crash to any lane reading the status.
pub fn emit_generated(aeon: &Path) -> Result<(), String> {
    emit_generated_in(aeon, &aeon.join(TREE_SOUND_MOUNT), None).map(|_| ())
}

/// Emit every generated sound artifact of `aeon` into `gen`, under an anchor overlay:
/// every artifact whose bytes derive from the bank anchors is emitted from the overlaid
/// ones. The seq-opcode table reads no anchor (its cells are resident handler VMAs), so
/// it has no overlay variant.
///
/// A build passes its generated root directory ([`sigil_build::build_dir`]), outside
/// the tree, and its `.emp` lowering reads the files back through that root's mount, so
/// the tree is only read. [`emit_generated`] passes the tree's own mount.
///
/// Returns seam 1's two resident links ([`seam1::ResidentBlobs`]): the blob leaves this
/// process as a file and re-enters the ROM as embedded bytes, so the driver's symbols
/// and line rows reach the build export only through this return (G0a spec 4.4,
/// [`crate::export_z80`]).
pub fn emit_generated_in(
    aeon: &Path,
    gen: &Path,
    ov: Option<&crate::map_placement::AnchorOverlay>,
) -> Result<seam1::ResidentBlobs, String> {
    seam2::require_reference_tree_for(aeon, gen).map_err(|e| format!("emit_generated: {e}"))?;
    let resident =
        seam1::emit_sound_blob_in(aeon, ov, gen).map_err(|e| format!("emit_sound_blob (blob): {e}"))?;
    seam2::emit_dac_artifacts_in(aeon, ov, gen).map_err(|e| format!("emit_dac_artifacts: {e}"))?;
    seam2::emit_mt_artifacts_in(aeon, ov, gen).map_err(|e| format!("emit_mt_artifacts: {e}"))?;
    seam2::emit_sfx_artifacts_in(aeon, ov, gen).map_err(|e| format!("emit_sfx_artifacts: {e}"))?;
    seam2::emit_seq_opcode_artifacts(aeon, gen)
        .map_err(|e| format!("emit_seq_opcode_artifacts: {e}"))?;
    seam2::emit_sound_tables_artifacts_in(aeon, ov, gen)
        .map_err(|e| format!("emit_sound_tables_artifacts: {e}"))?;
    seam2::emit_pitchtable_artifacts_in(aeon, ov, gen)
        .map_err(|e| format!("emit_pitchtable_artifacts: {e}"))?;
    Ok(resident)
}

/// [`emit_generated`] for a TEST that wants a failure to be its own failure: the
/// gates run it as a precondition step, where a panic is the right outcome. It writes
/// the reference tree's mount, as [`emit_generated`] does; no build calls it.
pub fn ensure_generated(aeon: &Path) {
    emit_generated(aeon).unwrap_or_else(|e| panic!("{e}"));
}

/// The fold-vs-placement comparison over a resolved layout. `folded` is seam-2's
/// prediction for the shape, `(head label, folded base)` per bank, or `None` for a
/// sound-off shape, which folds nothing and must place none of
/// `seam2::FOLDED_BANK_HEADS`.
///
/// Per folded head, each case refuses under its own code:
///   - `[sound.fold-head-absent]`: no resolved section carries the label, so there is
///     no placed base to compare the folded one against.
///   - `[sound.fold-head-not-first]`: the label is placed but is not its section's head.
///     The packing walk aligns a section by its head label's declaration and seam-2
///     predicted this base by this label's, so the two no longer describe one base,
///     whatever the addresses happen to say.
///   - `[sound.fold-vs-placement]`: the head's placed base differs from the folded one.
///   - `[sound.fold-head-sound-off]`: a sound-off shape places a folded head, whose
///     pointers nothing in this shape predicted.
///
/// LMA, not vma_origin(): the sound-bank heads are phased (`vma: $8000`), so a VMA
/// read would hand back a window address and compare against nothing.
pub fn check_sound_fold(resolved: &[Section], folded: Option<&[(&str, u32)]>) -> Result<usize, String> {
    let placed = |label: &str| -> Option<(&Section, u32)> {
        resolved
            .iter()
            .find_map(|sec| sec.labels.iter().find(|l| l.name == label).map(|l| (sec, l.offset)))
    };

    let Some(folded) = folded else {
        for label in crate::seam2::FOLDED_BANK_HEADS {
            if let Some((sec, off)) = placed(label) {
                return Err(format!(
                    "[sound.fold-head-sound-off] this shape declares sound off, so seam-2 \
                     folded no pointers for it, yet its layout places `{label}` (section `{}`, \
                     {:#x}). The bytes under that label carry absolute pointers nothing in this \
                     shape predicted; the shape's sound flag and its registry disagree",
                    sec.name,
                    sec.lma + off
                ));
            }
        }
        return Ok(0);
    };

    let mut compared = 0;
    for &(label, predicted) in folded {
        let Some((sec, off)) = placed(label) else {
            return Err(format!(
                "[sound.fold-head-absent] seam-2 folded absolute pointers against `{label}` = \
                 {predicted:#x}, but no section in this sound-on layout carries that label, so \
                 the placed base cannot be compared with the folded one. The label is the \
                 section head seam-2 predicts under (seam2::FOLDED_BANK_HEADS); a rename or a \
                 removal of it must be made in both places"
            ));
        };
        let head = head_label(sec).unwrap_or("<none>");
        if head != label {
            return Err(format!(
                "[sound.fold-head-not-first] seam-2 folded absolute pointers against `{label}` \
                 = {predicted:#x} as the head of its section, but section `{}` is headed by \
                 `{head}` and places `{label}` at offset {off:#x}. The packing walk aligns the \
                 section by `{head}`'s declared alignment while seam-2 predicted by `{label}`'s, \
                 so the fold and the placement no longer describe one base. Keep `{label}` the \
                 first item of its section",
                sec.name
            ));
        }
        let actual = sec.lma + off;
        if actual != predicted {
            return Err(format!(
                "[sound.fold-vs-placement] seam-2 folded absolute pointers against \
                 `{label}` = {predicted:#x} but the build placed it at {actual:#x} \
                 (delta {:+}). Every pointer cell in that blob is off by the same \
                 amount, so the sound would be silent or garbled at runtime with no \
                 other symptom. The walk aligns this section's base to its DECLARED \
                 alignment (its `[[align]]` row in engine.toml or game.toml, else the \
                 evenness default) through sigil_build::align::AlignTable::packed_base; \
                 seam2::sound_layout must reach the same base through the same function \
                 with the same head label. Fix the prediction, not the declaration.",
                actual as i64 - predicted as i64
            ));
        }
        compared += 1;
    }
    Ok(compared)
}
