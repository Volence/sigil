//! Which of this repo's test binaries ask the reference tree a question — derived from
//! source, never written down.
//!
//! ONE derivation with several consumers, and that is the point. A count typed into a
//! file is a copied expectation that rots the first time a port gate is added; a count
//! derived twice is two derivations to keep in step. So the population is computed here
//! and read by:
//!
//!   * `crates/sigil-cli/tests/reference_dependence_is_named.rs` — the gate that makes a
//!     run say how much of itself it did not measure;
//!   * `test_support`'s bare-run refusal, which names the same population in the message
//!     that stops a run nobody gave a reference tree.
//!
//! A derivation that silently returns nothing would report a perfectly measured suite —
//! this module's own failure mode arriving one level up — so every consumer is expected to
//! carry a positive control against [`FLOOR`].

use std::path::{Path, PathBuf};

/// The guards every reference-dependent gate opens with. All three live in
/// [`crate::test_support`], and a file that calls any of them is asking the reference tree
/// a question.
///
/// `scripts/nightly_source_gates.sh` derives the same set by closure over
/// `test_support.rs`; `crates/sigil-harness/tests/source_gate_classification.rs` holds
/// that closure to this declaration, so the script and this list cannot drift apart
/// silently — a closure that stopped reaching one of these would make some file look like
/// it reads nothing, which is the one direction in which being wrong is quiet.
pub const GUARDS: [&str; 3] = ["reference_tree(", "reference_tree_for_profile(", "aeon_dir("];

/// The floor a positive control holds the derivation to.
///
/// Deliberately far below the measured population (40 binaries on 2026-08-30, and the
/// SUITE_PATHS routing raised it) so ordinary churn never trips it, while a broken walk
/// cannot pass. A zero here would render an unmeasured suite as a fully measured one.
pub const FLOOR: usize = 20;

/// The file whose own text names the guards in order to look for them, and which would
/// therefore find itself.
const SELF_NAMING: [&str; 1] = ["reference_dependence_is_named"];

/// This workspace's root, from this crate's compile-time manifest directory.
pub fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/
    p.pop(); // the workspace
    p
}

/// Every test binary whose body asks the reference tree a question, derived from source.
///
/// Sorted, so two callers comparing populations compare the same list.
pub fn reference_dependent_binaries(ws: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for crate_dir in std::fs::read_dir(ws.join("crates")).into_iter().flatten().flatten() {
        let tests = crate_dir.path().join("tests");
        for e in std::fs::read_dir(&tests).into_iter().flatten().flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            if SELF_NAMING.contains(&name.as_str()) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            if GUARDS.iter().any(|g| text.contains(g)) {
                out.push(name);
            }
        }
    }
    out.sort();
    out
}
