//! Spec §8.4 deliverable: the bounded multi-pass loop reports a non-convergence
//! diagnostic (rather than looping forever) on a synthetic never-stabilizing
//! input, and an ordinary forward reference DOES converge.

use sigil_frontend_as::{assemble, Options};

#[test]
fn never_stabilizing_input_hits_the_pass_cap_and_diagnoses() {
    // A genuine two-state oscillator built from a size-feedback loop:
    //
    //   if A = 0        ; emit two bytes only while A is currently 0
    //   db 0,0
    //   endif
    //   B:              ; B's offset is 0 or 2 depending on the emit above
    //   A = B           ; A takes B's (position-dependent) value each pass
    //
    // The seed-based loop feeds each pass the previous pass's symbols, so:
    //   pass 0: seed empty → `A = 0` folds Poison → arm false → B at 0 → A = 0
    //   pass 1: A = 0      → arm TRUE  → db 0,0 → B at 2 → A = 2
    //   pass 2: A = 2      → arm false → B at 0 → A = 0
    //   pass 3: A = 0      → arm TRUE  → B at 2 → A = 2  ...
    // A flip-flops 0,2,0,2,… so no pass ever equals its predecessor and the loop
    // exhausts its 8-pass cap. (A plain self-increment like `N = N + 1` instead
    // CONVERGES: with no seeded start it folds to Poison and is never defined —
    // this size-feedback oscillator is what actually exercises the cap.)
    let src = "        cpu z80\n        phase 0\n        if A = 0\n        db 0,0\n        endif\nB:\nA = B\n";
    let err = assemble(src, &Options::default()).expect_err("must not converge");
    assert!(
        err.iter().any(|d| d.message.to_lowercase().contains("converge")),
        "expected a non-convergence diagnostic, got: {err:?}"
    );
}

#[test]
fn ordinary_forward_reference_converges() {
    // A normal forward ref stabilizes by pass 2 (Target resolves on pass 1).
    let src = "        cpu z80\n        phase 0\n        jr Target\nTarget: nop\n";
    assert!(assemble(src, &Options::default()).is_ok(), "forward ref should converge");
}
