//! THE MESSAGE `pins_rs_is_current` FAILS WITH, and the verdict behind it.
//!
//! `tests/repin_pins.rs::pins_rs_is_current` is REFERENCE-DEPENDENT: it needs the
//! sibling aeon tree to reach its panic at all, so nothing there can assert on the
//! words it prints. This file is HERMETIC. It builds both drift cases out of the
//! committed `src/pins.rs` itself, and asserts on the produced TEXT, because the
//! text is the subject: the gate was already correct and its message was not.
//!
//! WHAT WENT WRONG, so a later reader does not re-derive it. The verdict comes from
//! `strip_provenance` equality over the WHOLE rendered file; the count came from
//! `diff_pins`, which reads `pub const` declarations and nothing else. A dash sweep
//! rewrote 38 string literals in `repin.rs` that render `pins.rs` COMMENTS, drifting
//! 108 comment lines while every pin value stood still, and the gate reported the
//! two true facts as `STALE ... (0 changed pin(s))`. That pair reads as a
//! self-contradiction and cost a seat real time.
//!
//! THE TEMPTING FIX IS THE WRONG ONE. Narrowing the verdict to what `diff_pins`
//! models would make the message consistent by making that real staleness
//! invisible. [`the_whole_file_verdict_survives_a_comment_only_drift`] is the guard
//! against a later simplification, and it is the reason the strict comparison can be
//! left alone.

use std::path::Path;

use sigil_harness::repin::{diff_pins, drift_report, regenerate_command, stale_pins_message};

/// The real generated file. Using it rather than a hand-typed stand-in means the
/// fixtures below are the shape the gate actually compares.
fn committed() -> &'static str {
    include_str!("../src/pins.rs")
}

/// A stand-in build directory and aeon tree, so the asserted text is exact rather
/// than dependent on where this run happens to have been built.
fn where_() -> (&'static Path, &'static Path) {
    (Path::new("/build/dir"), Path::new("/aeon/tree"))
}

/// The committed file with ONE pin's value changed and nothing else touched:
/// returns `(constant name, mutated text)`.
///
/// The name is FOUND rather than hard-coded, so this fixture does not rot the next
/// time the manifest's first entry is renamed.
fn one_pin_moved() -> (String, String) {
    let line = committed()
        .lines()
        .find(|l| l.starts_with("pub const ") && l.contains("= 0x") && l.ends_with(';'))
        .expect("pins.rs must declare at least one plain hex pin");
    let name = line
        .strip_prefix("pub const ")
        .and_then(|r| r.split_once(':'))
        .map(|(n, _)| n.trim().to_string())
        .expect("a pub const line names its constant");
    let (head, _) = line.split_once("= 0x").expect("checked above");
    let moved = format!("{head}= 0xDEAD;");
    assert_ne!(line, moved, "the fixture must actually change the value");
    (name, committed().replacen(line, &moved, 1))
}

/// The committed file with ONE comment line rewritten and no declaration touched:
/// the shape of the sweep that produced the misleading message.
fn comment_only_drift() -> String {
    let old = "//! GENERATED FILE, DO NOT EDIT BY HAND.";
    assert!(committed().contains(old), "the header comment must be present to mutate");
    committed().replacen(old, "//! GENERATED FILE. Regenerate it, never hand edit it.", 1)
}

// ── The verdict ─────────────────────────────────────────────────────────────

/// THE CASE A NARROWED VERDICT WOULD LOSE. A comment-only drift moves no pin value
/// and is still staleness: the committed file is not what the generator emits.
///
/// If a later session "simplifies" `drift_report` to ask `diff_pins` instead of
/// comparing the whole text, this reds. That is the whole point of it.
#[test]
fn the_whole_file_verdict_survives_a_comment_only_drift() {
    let generated = comment_only_drift();
    assert!(
        diff_pins(committed(), &generated).is_empty(),
        "the fixture must move NO pin value, or it proves nothing about the comment case"
    );
    let report = drift_report(committed(), &generated)
        .expect("a comment-only drift is STALE; a verdict that returns None here is weakened");
    assert!(report.pin_changes.is_empty(), "no pin moved in this fixture");
    assert!(report.other_lines() > 0, "the report must name what did differ");
}

/// A rebuild that moves no pin changes only the `[provenance]` stamp, and that is
/// NOT drift. The strict verdict has to keep this exemption or every rebuild reds.
#[test]
fn a_provenance_only_difference_is_not_drift() {
    let generated = committed().replacen(
        "//! [provenance] plain:",
        "//! [provenance] plain: rebuilt-elsewhere",
        1,
    );
    assert_ne!(committed(), generated.as_str(), "the fixture must change the text");
    assert!(
        drift_report(committed(), &generated).is_none(),
        "a provenance-only rebuild must not read as staleness"
    );
}

/// The file as committed is current against itself. A verdict that cannot say
/// "current" would red every run.
#[test]
fn an_identical_render_is_current() {
    assert!(drift_report(committed(), committed()).is_none());
}

// ── The message ─────────────────────────────────────────────────────────────

/// A real pin move: the message counts it, names the constant, and shows both
/// values. It must NOT carry the comment-case wording.
#[test]
fn a_moved_pin_value_is_counted_and_named() {
    let (name, generated) = one_pin_moved();
    let report = drift_report(committed(), &generated).expect("a moved pin value is drift");
    let (build, aeon) = where_();
    let msg = stale_pins_message(&report, Some(build), Some(aeon));

    assert!(
        msg.contains("WHAT DIFFERS: 1 pin value moved; the surrounding text is identical"),
        "the headline must cover BOTH comparisons, got:\n{msg}"
    );
    assert!(
        msg.contains("pin values that moved (name: committed -> regenerated):"),
        "the moved pins must be listed under their own heading, got:\n{msg}"
    );
    assert!(msg.contains(&format!("  {name}: ")), "the constant must be named, got:\n{msg}");
    assert!(msg.contains("0xDEAD"), "the new value must be shown, got:\n{msg}");
    // The matcher, not the guard: these two phrases belong to the OTHER case, and a
    // message that carries them here would pass a sloppier assertion.
    assert!(!msg.contains("NO pin value moved"), "wrong headline for a pin move:\n{msg}");
    assert!(
        !msg.contains("Every pin value in the committed file is still the value"),
        "the pins-stood-still explanation belongs to the comment case only:\n{msg}"
    );
}

/// A comment-only drift: the message says the pins all stood still, says how much
/// text moved, and says why that is still staleness. It must NOT carry the
/// pin-move wording, and it must not print a bare zero next to the word STALE.
#[test]
fn a_comment_only_drift_says_no_pin_value_moved_and_why() {
    let generated = comment_only_drift();
    let report = drift_report(committed(), &generated).expect("a comment-only drift is drift");
    let (build, aeon) = where_();
    let msg = stale_pins_message(&report, Some(build), Some(aeon));

    assert!(
        msg.contains(
            "WHAT DIFFERS: NO pin value moved; 2 line(s) of surrounding text differ \
             (1 committed-only, 1 regenerated-only)"
        ),
        "the headline must say the pins stood still AND how much text moved, got:\n{msg}"
    );
    assert!(
        msg.contains("Every pin value in the committed file is still the value the generator"),
        "the message must reconcile the two comparisons, got:\n{msg}"
    );
    assert!(
        msg.contains("do NOT narrow the comparison to pin values"),
        "the message must refuse the tempting fix in writing, got:\n{msg}"
    );
    assert!(
        msg.contains("surrounding text, committed side only (1 line(s)):")
            && msg.contains("//! GENERATED FILE, DO NOT EDIT BY HAND."),
        "the drifted line must be shown, got:\n{msg}"
    );
    assert!(
        msg.contains("surrounding text, regenerated side only (1 line(s)):")
            && msg.contains("//! GENERATED FILE. Regenerate it, never hand edit it."),
        "the replacement line must be shown, got:\n{msg}"
    );
    // The matcher, not the guard.
    assert!(
        !msg.contains("pin values that moved"),
        "there is no moved-pin list in this case:\n{msg}"
    );
    assert!(!msg.contains("1 pin value moved"), "wrong headline for a comment drift:\n{msg}");
}

/// A declaration whose VALUE stood still while its rendering moved is reported as
/// what it is, rather than being silently dropped (it is a `pub const` line, so the
/// surrounding-text bucket does not claim it either).
#[test]
fn a_reformatted_declaration_is_reported_rather_than_lost() {
    let (name, _) = one_pin_moved();
    let line = committed()
        .lines()
        .find(|l| l.starts_with(&format!("pub const {name}:")))
        .expect("the named constant has a line")
        .to_string();
    let generated = committed().replacen(&line, &format!("{line}   "), 1);
    let report = drift_report(committed(), &generated).expect("a respaced declaration is drift");
    assert!(report.pin_changes.is_empty(), "the VALUE did not move");
    assert_eq!(
        report.reformatted_declarations, 2,
        "one line on each side, both declarations, neither a value change"
    );
    let (build, aeon) = where_();
    let msg = stale_pins_message(&report, Some(build), Some(aeon));
    assert!(
        msg.contains("2 declaration line(s) differ without their value changing"),
        "a formatting-only declaration drift must be named, got:\n{msg}"
    );
}

/// The report never says STALE and then lists nothing. The only difference this
/// fixture carries is line ORDER, which every bucket is blind to, so the headline
/// has to say so in words.
#[test]
fn a_reordering_is_named_rather_than_reported_as_zeroes() {
    let mut lines: Vec<&str> = committed().lines().collect();
    let first = lines
        .iter()
        .position(|l| l.starts_with("pub const "))
        .expect("pins.rs declares constants");
    let second = lines[first + 1..]
        .iter()
        .position(|l| l.starts_with("pub const "))
        .map(|i| i + first + 1)
        .expect("pins.rs declares more than one constant");
    lines.swap(first, second);
    let generated = lines.join("\n");
    let report = drift_report(committed(), &generated).expect("a reordering is still drift");
    assert!(report.only_line_order_differs(), "the fixture adds and removes no line");
    let (build, aeon) = where_();
    let msg = stale_pins_message(&report, Some(build), Some(aeon));
    assert!(
        msg.contains("WHAT DIFFERS: the same lines in a different ORDER"),
        "a zero-difference report must explain itself, got:\n{msg}"
    );
}

// ── The remediation ─────────────────────────────────────────────────────────

/// THE PRINTED COMMAND MUST BE THE COMMAND THAT WORKS.
///
/// `cargo run -p sigil-harness --bin repin` on its own does not regenerate anything:
/// the resolve builds the sound-on shape, so with `SIGIL_EMIT` unset `repin` exits 2
/// and writes nothing. The gate used to print exactly that line and no more.
#[test]
fn the_remediation_carries_sigil_emit_and_the_emitter_build() {
    let (build, aeon) = where_();
    let cmd = regenerate_command(Some(build), Some(aeon));

    assert!(
        cmd.contains("SIGIL_EMIT=/build/dir/release/emit_sound_blob"),
        "the emitter path must be concrete, got:\n{cmd}"
    );
    assert!(cmd.contains("AEON_DIR=/aeon/tree"), "the reference tree must be named, got:\n{cmd}");
    assert!(
        cmd.contains("cargo build --release -p sigil-harness --bin emit_sound_blob"),
        "the emitter has to exist before it can be pointed at, got:\n{cmd}"
    );
    assert!(
        cmd.contains("cargo run --release -p sigil-harness --bin repin"),
        "the regeneration command itself must be present, got:\n{cmd}"
    );
    assert!(
        cmd.contains("SIGIL_EMIT IS PART OF THE COMMAND"),
        "the reader must be told why the extra variable is there, got:\n{cmd}"
    );
}

/// With nothing known, the remediation names the two variables as variables rather
/// than inventing a path. A wrong absolute path is worse than a placeholder.
#[test]
fn the_remediation_falls_back_to_named_placeholders() {
    let cmd = regenerate_command(None, None);
    assert!(cmd.contains("SIGIL_EMIT=$CARGO_TARGET_DIR/release/emit_sound_blob"), "{cmd}");
    assert!(cmd.contains("AEON_DIR=$AEON_DIR"), "{cmd}");
}

/// Every failing message the gate can produce carries the remediation, so no drift
/// case leaves the reader without the command.
#[test]
fn every_drift_case_prints_the_remediation() {
    let (_, moved) = one_pin_moved();
    let (build, aeon) = where_();
    for (case, generated) in [("moved pin", moved), ("comment only", comment_only_drift())] {
        let report = drift_report(committed(), &generated).expect("drift");
        let msg = stale_pins_message(&report, Some(build), Some(aeon));
        assert!(
            msg.contains("SIGIL_EMIT=/build/dir/release/emit_sound_blob"),
            "{case}: the remediation is missing from the message:\n{msg}"
        );
    }
}
