//! THE WRITE POPULATION, DERIVED FROM ITS OWN SOURCE.
//!
//! Two gates assert properties of "every emitter that writes into the reference
//! tree", and neither may carry its own idea of what that set is: a hand-kept list
//! goes stale by addition, and a gate covering six of seven emitters reports the
//! same green as one covering all seven. The set is therefore PARSED OUT OF
//! `native::ensure_generated`'s own body — the one place the emitters are enumerated
//! for real — so an emitter added there and not to a gate fails that gate by name.
//!
//! Shared rather than copied: two parsers of the same function are two things to
//! keep right, and the failure of the copy is silent.

use std::path::{Path, PathBuf};

/// The harness source `ensure_generated` lives in, resolved from the crate root
/// baked in at compile time.
pub fn native_rs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/native.rs")
}

/// The emitter names `ensure_generated` calls, read from its body. Every call in it
/// is spelled `seam1::emit_…(` / `seam2::emit_…(`, so the scan is over that shape
/// rather than over a list somebody maintains twice.
///
/// Every way this can fail to measure panics with `UNMEASURABLE` instead of
/// returning an empty set: an unreadable source, a renamed function, and a parse
/// that finds nothing all look exactly like a clean sweep over nothing.
pub fn emitters_named_by_ensure_generated() -> Vec<String> {
    let path = native_rs();
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "UNMEASURABLE: cannot read {} to derive the emitter set: {e}. A gate asserting a \
             property of every emitter `ensure_generated` drives does not know what it is \
             asserting over without this source, and must not report green.",
            path.display()
        )
    });

    let body = src
        .split_once("pub fn ensure_generated(aeon: &Path) {")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| {
            panic!(
                "UNMEASURABLE: no `pub fn ensure_generated(aeon: &Path) {{` in {}. The function \
                 the emitter set is derived from was renamed or re-signed; re-point the scan \
                 rather than letting it find nothing.",
                path.display()
            )
        });
    // The body ends at the first line that closes it at column 0.
    let body = body.split_once("\n}").map(|(b, _)| b).unwrap_or(body);

    let mut found = Vec::new();
    for seam in ["seam1::", "seam2::"] {
        let mut rest = body;
        while let Some(i) = rest.find(seam) {
            let after = &rest[i + seam.len()..];
            let name: String =
                after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if name.starts_with("emit_") && !found.contains(&name) {
                found.push(name);
            }
            rest = after;
        }
    }
    if found.is_empty() {
        panic!(
            "UNMEASURABLE: parsed 0 emitters out of `ensure_generated` in {}. A zero here reads \
             exactly like a clean run over nothing, so it is a failure, not a pass.",
            path.display()
        );
    }
    found
}

/// Coverage in BOTH directions between a gate's own arm table and the derived set:
/// an emitter with no arm is unmeasured, and an arm for an emitter that is no longer
/// driven makes the coverage count read higher than it is.
pub fn reconcile_arms(declared: &[String], exercised: &[&str]) {
    for name in declared {
        assert!(
            exercised.contains(&name.as_str()),
            "`ensure_generated` drives `{name}`, which this gate does not exercise. Its writes \
             into the reference tree are unmeasured — add it to the arm table. (derived from {})",
            native_rs().display()
        );
    }
    for name in exercised {
        assert!(
            declared.iter().any(|d| d == name),
            "the arm table lists `{name}`, which `ensure_generated` no longer drives. Drop the \
             arm or re-point the scan; a stale arm makes the coverage count read higher than it is."
        );
    }
}
