//! The boot read is bounded, and the bound is a standing property rather than an event.
//!
//! Ruled suite-wide 2026-09-02T17:45:03Z: every lane's own gate run fails when its overseer
//! boot document exceeds the byte bound, because a trim is a thing somebody did once while the
//! bound is a thing that must keep being true. This lane regrew 6 KB past it in four hours with
//! a status row still claiming it was under, which is the failure this gate exists to make
//! impossible.
//!
//! **The remedy when this fires is to MOVE CLOSED HISTORY OUT, never to shorten a rule.** The
//! owner's 2026-09-02T18:20:19Z ruling is explicit — *"over the bound, move history out in one
//! cut and carry on"* — and it forbids trimming by hand for the gate. A gate that pressured
//! toward shorter rules would make the document worse in exactly the way it is meant to protect
//! against, so the failure message says where the text goes.
//!
//! Bytes only, by the 2026-09-02T17:57:40Z ruling on this lane's own question: the bound exists
//! to make the boot read cheap and bytes are what a read costs. The line count is a guide,
//! reported as a residual, never gated.

use std::path::PathBuf;

/// The bound, in bytes. Suite-wide, not this lane's to pick.
const BOOT_READ_BYTES: u64 = 100_000;

fn boot_read() -> PathBuf {
    sigil_harness::reference_dependence::workspace_root().join("docs/OVERSEER.md")
}

/// Separated from the assertion so the positive control below can exercise the same predicate
/// on a size this repo does not have. A gate whose failing branch is never executed is a gate
/// nobody has seen work.
fn is_over(size: u64) -> bool {
    size > BOOT_READ_BYTES
}

#[test]
fn the_boot_read_is_inside_its_byte_bound() {
    let path = boot_read();
    let size = std::fs::metadata(&path)
        .unwrap_or_else(|e| panic!("cannot stat the boot read at {}: {e}", path.display()))
        .len();
    let lines = std::fs::read_to_string(&path).map(|s| s.lines().count()).unwrap_or(0);

    assert!(
        !is_over(size),
        "boot read {} is {size} B / {BOOT_READ_BYTES} B: OVER by {over} B ({lines} lines).\n\
         The remedy is ONE CUT that MOVES text out verbatim, to whichever of TWO destinations\n\
         its kind belongs, and leaves every rule in place:\n\
           - a rule read at a MOMENT (dispatching, landing, judging a green) -> OVERSEER-REFERENCE.md\n\
           - CLOSED HISTORY (a dated episode, a landing record) -> OVERSEER-LOG.md, under its\n\
             original line spans.\n\
         Both destinations, not the log alone. The 2026-09-09 cut measured only 4.5 kB of 22 kB as\n\
         genuinely closed history, so a log-only reading of this message either falls short of the\n\
         target or forces rules into the log, which the owner's split (by WHEN A RULE IS READ,\n\
         2026-09-04T15:38:47Z) forbids. This paragraph said 'the log' alone until that cut.\n\
         Do NOT shorten a rule to hit this number - the owner's ruling forbids it, and four of one\n\
         day's five additions here were corrections that had to be MORE precise than what they\n\
         replaced. Do NOT raise this bound: the maintenance edit that clears a legitimate red is\n\
         indistinguishable from hiding the defect.\n\
         Prove the cut lossless by set difference AND read every seam: the set difference is blind\n\
         to a sentence left behind whose antecedent moved out with its paragraph, and it cannot\n\
         tell a move from a deliberate rewrite, so declare any in-pass repair by hand.\n\
         scripts/boot_read_cut_lossless.py is half one; it needs OMD_BASE=<rev before the cut>\n\
         and refuses a run that weighed no text.",
        path.display(),
        over = size - BOOT_READ_BYTES
    );

    // The line count is a guide and is deliberately not gated (2026-09-02T17:57:40Z).
    println!("boot read docs/OVERSEER.md {size} B / {BOOT_READ_BYTES} B: inside ({lines} lines)");
}

#[test]
fn the_bound_actually_refuses_something() {
    // The positive control. Without it, `is_over` returning false for every input this repo
    // ever hands it would look identical to a gate that works.
    assert!(!is_over(BOOT_READ_BYTES), "a file exactly at the bound is inside it");
    assert!(is_over(BOOT_READ_BYTES + 1), "one byte over the bound must be refused");
    assert!(is_over(200_000), "a document twice the bound must be refused");
}
