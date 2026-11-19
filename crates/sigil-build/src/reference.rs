//! The aeon REFERENCE tree for this crate's reference-dependent tests: a mirror of
//! sigil-harness `test_support::aeon_dir` (the A8 / d-18 guard), which this crate may not
//! import (sigil-build never depends on sigil-harness).
//!
//! It lives under `src/` because the reference-tree variables may only be read there
//! (`sigil-harness/tests/reference_env_read_is_routed.rs`: no file under `crates/*/tests/`
//! reads them). It is test support, hidden from the docs, and no build path calls it.
//!
//! The semantics are test_support's, step for step:
//!
//!   1. `AEON_DIR`, when set and non-empty. Set but not a directory is a hard error at
//!      this step, never a fall-through.
//!   2. `EMPYREAN_SUITE_ROOT` joined with `aeon`, same rule.
//!   3. Not taken: a derived checkout is the owner's live tree, which is not a reference
//!      tree (test_support declines it too).
//!   4. Nobody named a tree: a bare run STOPS (d-18). `SIGIL_ALLOW_PARTIAL=1` takes the
//!      declared partial run instead and returns [`NO_REFERENCE_TREE`], so every
//!      reference-dependent row skips naming that path; it may not be combined with
//!      `SIGIL_STRICT_GATE`.
//!
//! A tree's CONTENTS are the per-test guard's question (`reference_tree` in each test
//! file), asked against the paths that test reads.

use std::path::PathBuf;

/// The checkout variable (SUITE_PATHS step 1). Same spelling as test_support's `AEON_DIR_VAR`.
pub const AEON_DIR_VAR: &str = "AEON_DIR";
/// The suite root (SUITE_PATHS step 2). Same spelling as test_support's `SUITE_ROOT_VAR`.
pub const SUITE_ROOT_VAR: &str = "EMPYREAN_SUITE_ROOT";
/// The declared-partial-run opt-in. Same spelling as test_support's `ALLOW_PARTIAL_VAR`.
pub const ALLOW_PARTIAL_VAR: &str = "SIGIL_ALLOW_PARTIAL";
/// The stand-in a partial run resolves to. Same spelling as test_support's.
pub const NO_REFERENCE_TREE: &str = "/nonexistent/SIGIL_ALLOW_PARTIAL-no-reference-tree-was-named";

fn set(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|v| !v.is_empty())
}

/// The aeon reference tree, by steps 1, 2 and 4 above. Announced once on stderr.
pub fn aeon_dir() -> PathBuf {
    let (path, step) = if let Some(v) = set(AEON_DIR_VAR) {
        let p = PathBuf::from(&v);
        assert!(
            p.is_dir(),
            "{AEON_DIR_VAR}={v} does not name a directory. A checkout variable that is set but wrong \
             is a hard error at its own step (SUITE_PATHS), not a null that lets {SUITE_ROOT_VAR} \
             answer in its place."
        );
        (p, "1: AEON_DIR")
    } else if let Some(root) = set(SUITE_ROOT_VAR) {
        let p = PathBuf::from(&root).join("aeon");
        assert!(
            p.is_dir(),
            "{SUITE_ROOT_VAR}={root} has no `aeon` directory. A suite root that is set but wrong is \
             a hard error at its own step (SUITE_PATHS)."
        );
        (p, "2: EMPYREAN_SUITE_ROOT")
    } else {
        let partial = set(ALLOW_PARTIAL_VAR).is_some();
        // Read directly, as test_support's resolver does: a consultation from inside the
        // resolver is not a strict-gated test body.
        let strict = std::env::var_os("SIGIL_STRICT_GATE").is_some();
        assert!(
            !(partial && strict),
            "{ALLOW_PARTIAL_VAR} and SIGIL_STRICT_GATE are both set and no reference tree is named. \
             A strict run may not skip a gate, so it cannot also be the partial one. Name a tree \
             with {AEON_DIR_VAR}, or drop one flag."
        );
        assert!(
            partial,
            "NO REFERENCE TREE IS NAMED, so this run can measure nothing it could attribute, and \
             STOPS. Name a provisioned tree, {AEON_DIR_VAR}=<aeon checkout>, or {SUITE_ROOT_VAR}=<the \
             directory holding the suite>, or declare a partial run with {ALLOW_PARTIAL_VAR}=1, in \
             which case every reference-dependent row is left unmeasured. Ruled d-18 \
             (docs/OVERSEER-REFERENCE.md, 2026-09-02)."
        );
        (PathBuf::from(NO_REFERENCE_TREE), "partial run (SIGIL_ALLOW_PARTIAL)")
    };
    static ANNOUNCE: std::sync::Once = std::sync::Once::new();
    ANNOUNCE.call_once(|| eprintln!("sigil-build reference tree: {} (step {step})", path.display()));
    path
}
