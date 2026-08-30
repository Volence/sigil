//! A READ-SIDE GATE MUST NOT MANUFACTURE THE TREE IT READS.
//!
//! Every sound emitter `native::ensure_generated` drives writes into
//! `$AEON_DIR/engine/sound/generated` — a directory INSIDE the reference tree. An
//! emitter that creates that directory before establishing the tree is there does
//! not merely fail late: it makes `$AEON_DIR` itself exist. The suite's reference
//! guards probe roots (`if !aeon.exists()`), so a root conjured by one row flips
//! every such guard from "skip" to "run against an empty tree", and a run against an
//! absent tree stops being the same scenario the second time it is run. Measured on
//! one command: absent+pristine 0 failed exit 0; after the suite's own mkdir 53
//! failed exit 101; directory deleted 0 failed exit 0.
//!
//! This gate holds the property directly. It points every emitter at a path that
//! DOES NOT EXIST and requires two things of each: it refuses with an error naming
//! the absent tree, and afterwards the path still does not exist.
//!
//! WHAT THE EXPECTATION IS DERIVED FROM — never a pin, never one measurement. The
//! set of emitters under test is PARSED OUT OF `ensure_generated`'s own body in
//! `src/native.rs`. Adding an eighth emitter there and not here fails this gate by
//! name; it cannot be covered six-sevenths and read as green.
//!
//! HOW IT TELLS "NOTHING WAS CREATED" FROM "NOTHING RAN" — the two look identical
//! from the filesystem, and the second is the cheaper accident. Three separate
//! refusals, each panicking with UNMEASURABLE rather than passing:
//!
//!   * the source `ensure_generated` is parsed from must be readable, and must
//!     yield a non-empty emitter set;
//!   * every parsed emitter must have an arm in [`EXERCISED`], and every arm must
//!     have been invoked;
//!   * every invocation must have RETURNED — an `Err` is the pass condition, so an
//!     emitter that silently succeeded against a non-existent tree is a failure,
//!     not an absence of evidence.
//!
//! It needs no reference tree of its own and therefore never skips: it runs in
//! every `cargo test --workspace`, which is what `scripts/landing-run.sh` invokes.

use std::path::{Path, PathBuf};

/// The signature every artifact emitter shares: read `aeon`, write into `out_dir`.
type Emitter = fn(&Path, &Path) -> Result<(), String>;

/// The emitters this gate drives, keyed by the name they are called under in
/// `ensure_generated`. The KEYS are checked against the set parsed out of that
/// function, so this table cannot fall behind it silently.
const EXERCISED: &[(&str, Emitter)] = &[
    ("emit_sound_blob", sigil_harness::seam1::emit_sound_blob),
    ("emit_dac_artifacts", sigil_harness::seam2::emit_dac_artifacts),
    ("emit_mt_artifacts", sigil_harness::seam2::emit_mt_artifacts),
    ("emit_sfx_artifacts", sigil_harness::seam2::emit_sfx_artifacts),
    ("emit_seq_opcode_artifacts", sigil_harness::seam2::emit_seq_opcode_artifacts),
    ("emit_sound_tables_artifacts", sigil_harness::seam2::emit_sound_tables_artifacts),
    ("emit_pitchtable_artifacts", sigil_harness::seam2::emit_pitchtable_artifacts),
];

/// The harness source `ensure_generated` lives in, resolved from the crate root
/// baked in at compile time.
fn native_rs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/native.rs")
}

/// The emitter names `ensure_generated` calls, read from its body. Every call in it
/// is spelled `seam1::emit_…(` / `seam2::emit_…(`, so the scan is over that shape
/// rather than over a list somebody maintains twice.
fn emitters_named_by_ensure_generated() -> Vec<String> {
    let path = native_rs();
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "UNMEASURABLE: cannot read {} to derive the emitter set: {e}. This gate asserts a \
             property of every emitter `ensure_generated` drives; without its source it does not \
             know what it is asserting over and must not report green.",
            path.display()
        )
    });

    let body = src
        .split_once("pub fn ensure_generated(aeon: &Path) {")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| {
            panic!(
                "UNMEASURABLE: no `pub fn ensure_generated(aeon: &Path) {{` in {}. The function \
                 this gate derives its emitter set from was renamed or re-signed; re-point the \
                 scan rather than letting it find nothing.",
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

/// A path under the temp dir that this process has NOT created and no other run can
/// collide with. It is the stand-in for an `AEON_DIR` pointed somewhere absent.
fn absent_reference_tree(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is after the epoch")
        .as_nanos();
    let p = std::env::temp_dir().join(format!("sigil-absent-aeon-{}-{tag}-{nanos}", std::process::id()));
    assert!(!p.exists(), "the absent-tree stand-in {} already exists", p.display());
    p
}

/// No emitter creates anything under a reference tree it has not established is
/// there, and each refuses by NAMING the tree.
#[test]
fn no_emitter_creates_anything_under_an_absent_reference_tree() {
    let declared = emitters_named_by_ensure_generated();

    // Coverage, both directions, before anything runs: an emitter `ensure_generated`
    // drives with no arm here would go untested while the gate read green.
    for name in &declared {
        assert!(
            EXERCISED.iter().any(|(n, _)| n == name),
            "`ensure_generated` drives `{name}`, which this gate does not exercise. Its writes \
             into the reference tree are unmeasured — add it to EXERCISED. (derived from {})",
            native_rs().display()
        );
    }
    for (name, _) in EXERCISED {
        assert!(
            declared.iter().any(|d| d == name),
            "EXERCISED lists `{name}`, which `ensure_generated` no longer drives. Drop the arm \
             or re-point the scan; a stale arm makes the coverage count read higher than it is."
        );
    }

    let mut ran = 0usize;
    let mut offenders = Vec::new();
    for (name, emit) in EXERCISED {
        let aeon = absent_reference_tree(name);
        let out_dir = aeon.join("engine/sound/generated");

        let result = emit(&aeon, &out_dir);
        ran += 1;

        // The refusal itself. A success against a tree that is not there would mean
        // the emitter produced bytes from nothing.
        let err = match result {
            Err(e) => e,
            Ok(()) => {
                let _ = std::fs::remove_dir_all(&aeon);
                panic!(
                    "UNMEASURABLE: `{name}` reported success against the non-existent tree {}. \
                     Nothing could have been read; the gate cannot tell what it just measured.",
                    aeon.display()
                );
            }
        };

        // The property: the tree's root must still not exist. Recorded before the
        // cleanup, so a failure still reports and a pass leaves nothing behind.
        let created = aeon.exists();
        let _ = std::fs::remove_dir_all(&aeon);
        if created {
            offenders.push(format!(
                "{name} created {} (error was: {err})",
                aeon.display()
            ));
            continue;
        }

        // A refusal a reader cannot act on sends them hunting the wrong tree, which
        // is the failure mode that made this defect expensive to reconcile.
        assert!(
            err.contains(&aeon.display().to_string()),
            "`{name}` refused without naming the tree it wanted. Error: {err}\nExpected the \
             path {} to appear in it.",
            aeon.display()
        );
    }

    assert!(
        offenders.is_empty(),
        "{} of {} emitters created a path inside a reference tree that does not exist. Creating \
         it makes $AEON_DIR's root exist, which flips every root-probing skip guard in the suite \
         from `skip` to `run against an empty tree`:\n  {}",
        offenders.len(),
        EXERCISED.len(),
        offenders.join("\n  ")
    );

    // Positive witness that the run happened. Without it, an EXERCISED table that
    // somehow iterated zero times would report the same green as a clean sweep.
    assert_eq!(
        ran,
        declared.len(),
        "UNMEASURABLE: exercised {ran} emitters but `ensure_generated` drives {}. A count that \
         does not reconcile is a run that measured something other than the property.",
        declared.len()
    );
}

/// `ensure_generated` refuses at its own entry, so the refusal does not depend on
/// which emitter happens to be first in its body.
#[test]
fn ensure_generated_refuses_before_it_touches_an_absent_tree() {
    let aeon = absent_reference_tree("ensure-generated");

    let panicked = std::panic::catch_unwind(|| {
        sigil_harness::native::ensure_generated(&aeon);
    })
    .is_err();

    let created = aeon.exists();
    let _ = std::fs::remove_dir_all(&aeon);

    assert!(
        panicked,
        "UNMEASURABLE: `ensure_generated` returned normally against the non-existent tree {}. It \
         writes seven artifacts; a normal return means it did not run or did not write.",
        aeon.display()
    );
    assert!(
        !created,
        "`ensure_generated` created {} while refusing. The mkdir must follow the validation, not \
         precede it.",
        aeon.display()
    );
}
