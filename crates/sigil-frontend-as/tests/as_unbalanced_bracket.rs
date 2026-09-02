//! The unclosed-bracket landmine: this front end REFUSES what AS accepts.
//!
//! AS assembles `addq.b #2,obRoutine(a0` — a missing `)` — by treating the rest
//! of the line as an expression it can make sense of. The result is a typo that
//! builds, which is one of the specific complaints the AS-replacement project
//! exists to answer, so the divergence here is deliberate.
//!
//! THIS FILE EXISTS BECAUSE THE DIVERGENCE IS THE KIND SOMEBODY LATER "FIXES".
//! A future reader chasing AS compatibility meets a refusal where AS was happy,
//! reasonably concludes the front end is incomplete, and makes it accept — which
//! would restore the landmine while looking like progress. The decision is
//! recorded as a test rather than as prose so that reversing it has to be
//! deliberate: this file goes red, and the message says why.
//!
//! Not a claim that every AS bracket quirk is refused; it pins this one shape.

use sigil_frontend_as::{assemble, Options};

/// Enough preamble that a failure can only be about the operand under test.
fn src(tail: &str) -> String {
    format!("        cpu 68000\n        phase 0\nobRoutine = 4\n{tail}")
}

/// The controls are half the value: without them a refusal below could equally
/// be the harness failing to assemble anything at all, which is how the first
/// version of this probe read — every case "refused", including the ones that
/// must not, and the identical message across all of them was the tell.
#[test]
fn the_closed_forms_assemble() {
    for (label, tail) in [
        ("closed paren", "        addq.b #2,obRoutine(a0)\n"),
        ("no operand at all", "        nop\n"),
    ] {
        assert!(
            assemble(&src(tail), &Options::default()).is_ok(),
            "control `{label}` must assemble; if it does not, every refusal in \
             this file is about the harness rather than about brackets"
        );
    }
}

#[test]
fn an_unclosed_bracket_is_refused_by_name() {
    for (label, tail) in [
        ("bare", "        addq.b #2,obRoutine(a0\n"),
        ("with a trailing comment", "        addq.b #2,obRoutine(a0 ; note\n"),
    ] {
        let diags = assemble(&src(tail), &Options::default())
            .err()
            .unwrap_or_else(|| panic!("`{label}`: an unclosed bracket must be refused, not assembled — AS accepts this and that acceptance is the defect being diverged from"));

        // Matched on wording unique to THIS rule. A looser matcher would pass on
        // any refusal, including the "expected mnemonic or directive after label"
        // that the broken first version of this probe produced for every case.
        let joined = diags
            .iter()
            .map(|d| d.message.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(
            joined.contains("trailing tokens in operand"),
            "`{label}`: refused, but not for the bracket. Got: {joined}"
        );
    }
}
