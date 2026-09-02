//! The suite says how much of itself it did not measure.
//!
//! Queue row `APC-FIX`. The `absolute-path-classify` measurement (`8a377311`,
//! `docs/superpowers/notes/2026-08-30-absolute-path-classify.md`) established the defect
//! this gate exists for, and it is the sharpest silent-green in the repo:
//!
//! > with the reference tree entirely absent, the ordinary run is **4168 passed / 0 failed,
//! > exit 0** — a fully green suite that measured 398 fewer rows than it appears to.
//! > `SIGIL_STRICT_GATE=1` turns those same 398 into failures that name the path. Both facts
//! > are true of the same tree at the same moment; only the flag differs.
//!
//! So **the only thing standing between "no reference tree" and a green suite is a flag that
//! is set at landings and nowhere else.** Every one of those rows skips through
//! [`sigil_harness::test_support::reference_tree`], which prints one `skip:` line to stderr —
//! captured by the test harness, counted by nobody, and gone.
//!
//! ## What this gate does, and the line it deliberately does not cross
//!
//! It makes the degradation **say its own size**, and changes no lane's pass/fail. Whether the
//! ORDINARY suite should refuse to run without a reference tree is a different question with
//! cross-lane consequences — every lane running the suite bare would start getting refusals —
//! and it is the unfinished half of the owner's `d-17`, whose answer closed the WRITE side only
//! and explicitly scoped the first move to the 29 places that can write rather than all 127.
//! That half is his, it is filed, and it is not smuggled in here.
//!
//! The remedy is the one this lane applied twice on 2026-08-30 (the drift report's tree-state
//! fold, and the seam gate's dead clause): **name what you did not measure, rather than
//! rendering it as fine.** A number a reader sees on every run is not a refusal, and it is the
//! most this gate can do without taking a decision that is not its own.
//!
//! ## Why the population is derived and never written down
//!
//! A count typed into this file would be a copied expectation of exactly the kind this repo
//! rejects, and it would rot the first time a port gate was added. The reference-dependent set
//! is recomputed from sigil's own test sources on every run, and a POSITIVE CONTROL fails the
//! gate if that derivation ever stops finding anything — because a derivation that silently
//! returns zero would report a perfectly measured suite, which is this file's own failure mode
//! arriving one level up.

use sigil_harness::reference_dependence::{reference_dependent_binaries, workspace_root, FLOOR};

#[test]
fn the_suite_names_the_measurement_it_did_not_take() {
    let ws = workspace_root();
    let gated = reference_dependent_binaries(&ws);

    // POSITIVE CONTROL. A derivation that finds nothing would report a fully measured suite
    // — the exact reading this gate exists to prevent — and it would do so silently.
    assert!(
        gated.len() > FLOOR,
        "COULD NOT MEASURE: the reference-dependent derivation found only {} test \
         binaries, so its answer says nothing about how much of the suite is gated. \
         A zero here would render an unmeasured suite as a fully measured one.",
        gated.len()
    );

    // The RESOLVER is consulted rather than `aeon_dir()`: this gate's whole subject is the
    // state where no reference tree was named, and `aeon_dir()` is the function that acts
    // on that state. Asking it here would make the gate a consumer of the behaviour it
    // reports on.
    let aeon = match sigil_harness::test_support::aeon_checkout() {
        Ok(c) if c.step.names_a_reference_tree() => c.path,
        // Nobody named a tree: the state below, whether the resolver derived one or
        // refused outright.
        _ => std::path::PathBuf::from("(no reference tree was named)"),
    };
    let strict = std::env::var("SIGIL_STRICT_GATE").is_ok();

    if aeon.is_dir() {
        println!(
            "reference tree present at {} — the {} reference-dependent test \
             binaries are measuring against it",
            aeon.display(),
            gated.len()
        );
        return;
    }

    // THE STATE THIS GATE EXISTS FOR. Absent tree, ordinary mode: the suite is about to go
    // green having skipped every row in every one of these binaries.
    let banner = format!(
        "THE REFERENCE TREE IS ABSENT ({}). {} test binaries are reference-dependent and \
         every row in them will SKIP. A green result from this run does NOT mean those \
         rows passed — it means they were not run. Binaries: {}",
        aeon.display(),
        gated.len(),
        gated.join(", ")
    );

    assert!(
        !strict,
        "{banner}\n\nSIGIL_STRICT_GATE is set, so this is a failure rather than a notice. \
         The individual gates each fail naming their own missing path; this one names the \
         CAUSE those failures share, so a reader is not left inferring it from 398 rows."
    );

    // Ordinary mode: print, do not fail. Refusing here would change the suite's behaviour for
    // every lane that runs it bare, which is the owner's open half of `d-17` and not this
    // gate's to take.
    println!("{banner}");
    println!(
        "This run is NOT a landing. Set SIGIL_STRICT_GATE=1 (as scripts/landing-run.sh does) \
         to turn these skips into named failures."
    );
}
