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
//! while passing straight over this guard on their way there.
//!
//! TWO CONTRACTS, both proven here. PLACEMENT is the drift between the appendix start
//! and blob end. MEMBERSHIP is whether the blob label is there to measure at all, and it
//! is judged against the caller's own declaration of the shape rather than against the
//! listing — because an islandless listing is a satisfied contract for the one shape
//! that ships no island and a silent runtime failure for every other, and the listing
//! says the same thing in both cases. Both directions of the mismatch are refusals.
//! The per-shape half — that the shapes declaring the island are exactly the shapes
//! whose listings define its label — is `error_handler_island_membership.rs`.
//!
//! These probes need no built engine tree on disk. The guard is a precondition decided
//! from the listing plus the caller's declaration, and it runs BEFORE
//! `append_deb2_appendix` looks for `convsym` — which is also what makes the control
//! arms below meaningful: when the contracts HOLD, the call proceeds and fails on the
//! missing tool instead, so a refuse-everything guard cannot satisfy this file.

use sigil_harness::native::{
    self, ERROR_HANDLER_BLOB_LABEL, ERROR_HANDLER_BLOB_LEN, SONIC4_APPENDIX_FLOOR,
};
use sigil_link::ListingSymbol;
use std::path::Path;

/// Arbitrary — the guard compares the blob's own value against the ROM length, so no
/// shipped address is involved and this survives any re-layout.
const BLOB_VMA: u32 = 0x9_F000;

fn sym(name: &str, value: u32) -> ListingSymbol {
    ListingSymbol { name: name.to_string(), value, is_equate: false, unused: false, lma: None }
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
/// `expect_island` is the shape's own declaration of whether it carries the island,
/// which every probe below states explicitly because it is the half of the contract the
/// listing cannot supply.
fn appendix_starting_at(
    rom_len: usize,
    listing: &[ListingSymbol],
    expect_island: bool,
) -> Result<Vec<u8>, String> {
    let rom = vec![0u8; rom_len];
    native::append_deb2_appendix(
        Path::new("/nonexistent-engine-tree-for-this-probe"),
        &rom,
        listing,
        false,
        SONIC4_APPENDIX_FLOOR,
        expect_island,
    )
}

fn blob_end() -> usize {
    BLOB_VMA.wrapping_add(ERROR_HANDLER_BLOB_LEN) as usize
}

#[test]
fn a_section_emitted_after_the_blob_is_refused_by_name() {
    let err = appendix_starting_at(blob_end() + 2, &listing_with_blob(), true)
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
    let err = appendix_starting_at(blob_end() - 2, &listing_with_blob(), true)
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
    let err = appendix_starting_at(blob_end(), &listing_with_blob(), true)
        .expect_err("no engine tree on disk, so the call still fails, but LATER");
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

/// A missing blob label under a shape that DECLARES the island is a refusal, not a
/// skipped check.
///
/// Nothing in a listing distinguishes "this shape ships no island" from "the label was
/// renamed / the island stopped being placed / the harvest dropped the row". Judging it
/// as the first is a build that succeeds and a crash screen that prints `<unknown>` for
/// every line, so the caller states which shape it is holding and this arm refuses the
/// mismatch. Note the placement here is ALSO wrong by two bytes — the membership fault
/// is reported first because there is no subject to measure placement against.
#[test]
fn a_declared_island_with_no_blob_label_is_refused() {
    let no_blob = vec![sym("EntryPoint", 0x200), sym("GameLoop", 0x2_56E)];
    let err = appendix_starting_at(blob_end() + 2, &no_blob, true)
        .expect_err("a declared island whose label never appeared must be refused");
    assert!(
        err.contains("MDDBG island MEMBERSHIP violated"),
        "must fail as the membership rule, which is the fault that actually happened; \
         got: {err}"
    );
    assert!(
        err.contains("<unknown>"),
        "must name the silent runtime symptom, since nothing else reports it; got: {err}"
    );
}

/// The other direction, and the reason the probe above is an assertion rather than a
/// blanket refusal of islandless listings: a shape that declares NO island passes the
/// same listing straight through to the rest of the pipeline.
#[test]
fn an_undeclared_island_lets_an_islandless_listing_through() {
    let no_blob = vec![sym("EntryPoint", 0x200), sym("GameLoop", 0x2_56E)];
    let err = appendix_starting_at(blob_end() + 2, &no_blob, false)
        .expect_err("no engine tree on disk, so the call still fails, but LATER");
    assert!(
        !err.contains("MDDBG"),
        "a shape with no island has no MDDBG contract to break; got: {err}"
    );
    assert!(
        err.contains("convsym not found"),
        "the call must have proceeded PAST the precondition; got: {err}"
    );
}

/// The blob label appearing in a shape that declares no island is equally a refusal —
/// the registry's fault-handler split is exclusive, so this shape is carrying an island
/// nothing placed.
#[test]
fn an_undeclared_island_whose_label_appears_is_refused() {
    let err = appendix_starting_at(blob_end(), &listing_with_blob(), false)
        .expect_err("an island label under a shape that declares none must be refused");
    assert!(
        err.contains("MDDBG island MEMBERSHIP violated"),
        "must fail as the membership rule; got: {err}"
    );
    assert!(
        err.contains("0x9f000"),
        "must name where the unexpected label sits, so the reader can find it; got: {err}"
    );
}
