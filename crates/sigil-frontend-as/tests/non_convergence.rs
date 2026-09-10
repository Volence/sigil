//! Spec §8.4 deliverable, restated under owner ruling `d-23-answered`: an input
//! that never settles is DIAGNOSED rather than looped on forever, an ordinary
//! forward reference converges, and what the author is told is which symbols
//! are moving and between which values.
//!
//! ## What changed here, and why the old assertion had to go
//!
//! This file used to be called `never_stabilizing_input_hits_the_pass_cap_and_
//! diagnoses` and it asserted that the message contained the word "converge",
//! which in practice meant `assembly did not converge within 16 passes`. That
//! string is the exact thing the ruling removes: a pass count describes the
//! assembler's own effort and tells the author nothing they can act on, and it
//! is what the Sonic community named when asked what it dislikes about AS.
//!
//! **The old assertion is not merely renamed, it is inverted.** The test now
//! asserts that no count of attempts appears in any wording, so the message this
//! file used to demand would fail it. That is deliberate: an assertion that
//! merely stopped checking for the old string would let it come back.
//!
//! The input is unchanged, and it is answered better than before. It is a
//! genuine period-two oscillation, so the fixpoint loop now proves that no
//! fixpoint exists (the pass function is deterministic, and an environment equal
//! to one from two or more passes back repeats forever) instead of running out
//! of a budget. It reaches its verdict earlier AND says more.

use sigil_frontend_as::{assemble, Options};

/// A size-feedback oscillator:
///
/// ```text
///   if A = 0        ; emit two bytes only while A is currently 0
///   db 0,0
///   endif
///   B:              ; B's offset is 0 or 2 depending on the emit above
///   A = B           ; A takes B's (position-dependent) value each pass
/// ```
///
/// The seed-based loop feeds each pass the previous pass's symbols, so `A`
/// flip-flops 0, 2, 0, 2 and no pass ever equals its predecessor. (A plain
/// self-increment like `N = N + 1` instead CONVERGES: with no seeded start it
/// folds to Poison and is never defined, so this size-feedback shape is what
/// actually exercises the path.)
///
/// Not refused as circular, and that is the split working. The condition here is
/// an `if`, whose two arms are two settled states rather than a size that grows
/// out of its own value, so it is the half of the ruling that reports rather
/// than the half that refuses.
#[test]
fn a_never_settling_input_names_the_moving_symbols_and_their_values() {
    let src = "        cpu z80\n        phase 0\n        if A = 0\n        db 0,0\n        endif\nB:\nA = B\n";
    let err = assemble(src, &Options::default()).expect_err("must not settle");
    let text = err
        .iter()
        .map(|d| d.message.clone())
        .collect::<Vec<_>>()
        .join(" || ");
    assert!(text.contains("never settles"), "{text}");
    // What is moving, by name, and what it moves between. Both symbols in the
    // loop and both of the two values each takes.
    assert!(text.contains("`A`"), "{text}");
    assert!(text.contains("`B`"), "{text}");
    assert!(text.contains('0') && text.contains('2'), "{text}");
    // The ruling, asserted directly. A pass count is never shown, in any
    // wording, and the old message would fail every one of these.
    assert!(!text.contains("16"), "a pass count leaked into the message: {text}");
    assert!(!text.contains("pass"), "a pass count leaked into the message: {text}");
    assert!(!text.contains("converge"), "the old wording survived: {text}");
}

#[test]
fn ordinary_forward_reference_converges() {
    // A normal forward ref stabilizes by pass 2 (Target resolves on pass 1).
    let src = "        cpu z80\n        phase 0\n        jr Target\nTarget: nop\n";
    assert!(
        assemble(src, &Options::default()).is_ok(),
        "forward ref should converge"
    );
}
