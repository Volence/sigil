//! The MDDBG blob-end contract, proven to FIRE.
//!
//! `native::check_error_handler_is_last` asserts the deb2 appendix begins at exactly
//! `ErrorHandlerBlob + ERROR_HANDLER_BLOB_LEN`. The vendored MD Debugger blob locates its
//! symbol table through two PC-relative `lea` displacements baked into its opaque bytes,
//! both pointing one past the blob's last byte, so anything emitted between the blob and
//! the appendix breaks runtime symbol resolution SILENTLY — the build succeeds and every
//! crash-screen backtrace line prints `<unknown>`. A guard against a silent failure is
//! worth only what its proof of firing is worth, and until this file that proof did not
//! exist: nothing in the workspace planted a section after the blob, and the two
//! neighbouring negative controls in `native_full_rom.rs` target the appendix SIZE BAND
//! while taking this guard's vacuous arm on their way past it.
//!
//! These probes need no built engine tree on disk. The guard is a placement precondition
//! decided from the listing alone, and it runs BEFORE `append_deb2_appendix` looks for
//! `convsym` — which is also what makes the control arm below meaningful: when the
//! contract HOLDS, the call proceeds and fails on the missing tool instead, so a
//! refuse-everything guard cannot satisfy this file.

use sigil_harness::native::{
    self, ERROR_HANDLER_BLOB_LABEL, ERROR_HANDLER_BLOB_LEN, SONIC4_APPENDIX_FLOOR,
};
use sigil_link::ListingSymbol;
use std::path::Path;

/// Arbitrary — the guard compares the blob's own value against the ROM length, so no
/// shipped address is involved and this survives any re-layout.
const BLOB_VMA: u32 = 0x9_F000;

fn sym(name: &str, value: u32) -> ListingSymbol {
    ListingSymbol { name: name.to_string(), value, is_equate: false, unused: false }
}

/// A listing shaped like a real one: the blob plus ordinary neighbours, so a guard that
/// merely found *some* symbol would not pass for the wrong reason.
fn listing_with_blob() -> Vec<ListingSymbol> {
    vec![
        sym("EntryPoint", 0x200),
        sym("GameLoop", 0x2_56E),
        sym(ERROR_HANDLER_BLOB_LABEL, BLOB_VMA),
    ]
}

/// The appendix start IS the ROM length — that is what the guard is handed.
fn appendix_starting_at(rom_len: usize, listing: &[ListingSymbol]) -> Result<Vec<u8>, String> {
    let rom = vec![0u8; rom_len];
    native::append_deb2_appendix(
        Path::new("/nonexistent-engine-tree-for-this-probe"),
        &rom,
        listing,
        false,
        SONIC4_APPENDIX_FLOOR,
    )
}

fn blob_end() -> usize {
    BLOB_VMA.wrapping_add(ERROR_HANDLER_BLOB_LEN) as usize
}

#[test]
fn a_section_emitted_after_the_blob_is_refused_by_name() {
    let err = appendix_starting_at(blob_end() + 2, &listing_with_blob())
        .expect_err("two bytes after the blob must fail the blob-end contract");
    assert!(
        err.contains("blob-end contract VIOLATED"),
        "must fail as THIS rule, not as some neighbouring check; got: {err}"
    );
    assert!(
        err.contains("+2"),
        "the drift must be reported signed and exact, so the reader knows how far; got: {err}"
    );
    assert!(
        err.contains("placed AFTER the blob"),
        "the AFTER direction must name the consequence (the blob reads the intruding \
         bytes and every backtrace line prints `<unknown>`); got: {err}"
    );
}

#[test]
fn an_appendix_starting_before_blob_end_is_refused_with_the_other_diagnosis() {
    let err = appendix_starting_at(blob_end() - 2, &listing_with_blob())
        .expect_err("an appendix inside the blob must fail the blob-end contract");
    assert!(
        err.contains("blob-end contract VIOLATED"),
        "must fail as THIS rule; got: {err}"
    );
    assert!(
        err.contains("-2") && err.contains("truncated"),
        "the BEFORE direction is a different fault (short blob / stale length) and must \
         not be reported as an intruding section; got: {err}"
    );
}

/// The control, and the reason the two probes above are not vacuous: at the CORRECT
/// placement the guard must let the call through. A guard that refused everything would
/// satisfy both probes and fail here.
#[test]
fn the_exact_blob_end_placement_passes_the_guard_and_proceeds() {
    let err = appendix_starting_at(blob_end(), &listing_with_blob())
        .expect_err("no engine tree on disk, so the call still fails — but LATER");
    assert!(
        !err.contains("blob-end contract"),
        "the contract holds at exactly blob end and must not fire; got: {err}"
    );
    assert!(
        err.contains("convsym not found"),
        "the call must have proceeded PAST the placement precondition to the tool \
         lookup, which is what proves the guard is not a blanket refusal; got: {err}"
    );
}

/// The fail-open, asserted so it has a name and a subject rather than blessing it.
///
/// A listing carrying no blob label returns `Ok(())` from the placement check. That is
/// correct for a shape genuinely without the island, and INDISTINGUISHABLE from a
/// renamed label or a harvest that dropped it for a shape that has one. Closing it means
/// asserting, per shape, that every shape carrying the island reaches the non-vacuous
/// arm; that lands separately, because it changes `native.rs`.
#[test]
fn a_listing_without_the_blob_label_skips_the_check_entirely() {
    let no_blob = vec![sym("EntryPoint", 0x200), sym("GameLoop", 0x2_56E)];
    let err = appendix_starting_at(blob_end() + 2, &no_blob)
        .expect_err("no engine tree on disk, so the call still fails — but LATER");
    assert!(
        !err.contains("blob-end contract"),
        "the check is vacuous without the blob label — the placement above is WRONG by \
         two bytes and goes unreported; got: {err}"
    );
}
