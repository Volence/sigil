//! `rept` shares the bounded-loop-or-diagnose contract `while` already has: the
//! TOTAL number of `rept` body executions in one pass is budgeted, and a pass
//! that exhausts it is refused, naming the `rept` line, in bounded time.
//!
//! ## What was silently wrong
//!
//! ```text
//!     rept    100000
//!     rept    100000
//! x   set     x+1
//!     endr
//!     endr
//! ```
//!
//! ran ten billion body executions. Nothing diagnosed and nothing terminated:
//! `exec_rept` folded its count once and looped, with neither the per-loop cap
//! nor the per-pass budget `exec_while` checks twelve screens below it. A build
//! carrying this shape "timed out" wherever it ran, which is a silence with a
//! different name.
//!
//! ## The reference
//!
//! asl (reference build md5 `61e672562465725a8c102288a7da9098`, exit status
//! checked) has no budget either: on the source above, and on the same nesting
//! with a comment as the only body line, it ran past a 30-second timeout with
//! no output. That is a finding about asl, not a behaviour to match. Two things
//! it does that ARE matched here: a nested `rept 300 / rept 300 / x set x+1`
//! assembles to `dc.l x` = `00 01 5F 90` (90000, exit 0), and a nested pair of
//! `rept 100000` with NO body line at all finishes in 80 ms with exit 0
//! (`dc.b 1` after it emits `01`), so a body with no lines costs no budget.

use sigil_frontend_as::{assemble, Options};
use std::time::Duration;

/// The bound a run must finish inside for the budget to count as bounded time.
/// The pre-fix code does not finish inside it; that timeout IS the red.
const BOUND: Duration = Duration::from_secs(60);

/// Assemble on a worker thread and wait at most [`BOUND`]. A run that does not
/// come back in time fails naming the bound, rather than hanging the harness.
fn assemble_within_bound(src: &'static str) -> Result<Vec<u8>, Vec<String>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let r = assemble(src, &Options::default())
            .map(|m| m.sections.first().map(|s| s.image_bytes()).unwrap_or_default())
            .map_err(|ds| ds.into_iter().map(|d| d.message).collect::<Vec<_>>());
        let _ = tx.send(r);
    });
    match rx.recv_timeout(BOUND) {
        Ok(r) => r,
        Err(_) => panic!("assembly did not terminate within {BOUND:?}"),
    }
}

/// The diagnostics of a run that must be refused. An accepted run fails naming
/// only the image SIZE: the pre-fix `rept 1000001` emits a megabyte.
fn refusal(src: &'static str) -> Vec<String> {
    match assemble_within_bound(src) {
        Ok(bytes) => panic!("assembled with exit 0 to {} byte(s) instead of refusing", bytes.len()),
        Err(diags) => diags,
    }
}

#[test]
fn nested_rept_exceeding_the_budget_is_refused_in_bounded_time() {
    let src = "\tcpu 68000\n\tpadding off\nx\tset 0\n\trept 100000\n\trept 100000\nx\tset x+1\n\tendr\n\tendr\n\tdc.l x\n";
    let err = refusal(src);
    assert!(
        err.iter().any(|m| m.contains("rept") && m.contains("budget") && m.contains("1000000")),
        "expected a `rept` budget diagnostic naming the 1000000 cap; got {err:?}"
    );
}

#[test]
fn nested_rept_with_a_comment_body_is_refused_in_bounded_time() {
    // The body is one comment line: still a body execution per iteration.
    let src = "\tcpu 68000\n\tpadding off\n\trept 100000\n\trept 100000\n\t; nothing here\n\tendr\n\tendr\n\tdc.b 1\n";
    let err = refusal(src);
    assert!(
        err.iter().any(|m| m.contains("rept") && m.contains("budget")),
        "expected a `rept` budget diagnostic; got {err:?}"
    );
}

#[test]
fn nested_rept_with_no_body_line_costs_nothing_and_assembles() {
    // asl: exit 0 in 80 ms, image `01`.
    let src = "\tcpu 68000\n\tpadding off\n\trept 100000\n\trept 100000\n\tendr\n\tendr\n\tdc.b 1\n";
    assert_eq!(assemble_within_bound(src).expect("assemble"), vec![1]);
}

#[test]
fn nested_rept_within_the_budget_assembles_as_asl() {
    // asl: `00 01 5F 90`.
    let src = "\tcpu 68000\n\tpadding off\nx\tset 0\n\trept 300\n\trept 300\nx\tset x+1\n\tendr\n\tendr\n\tdc.l x\n";
    assert_eq!(assemble_within_bound(src).expect("assemble"), vec![0x00, 0x01, 0x5F, 0x90]);
}

#[test]
fn a_single_rept_over_the_budget_is_refused_naming_its_line() {
    let src = "\tcpu 68000\n\tpadding off\n\trept 1000001\n\tdc.b 7\n\tendr\n";
    let err = refusal(src);
    assert!(
        err.iter().any(|m| m.contains("rept") && m.contains("budget")),
        "expected a `rept` budget diagnostic; got {err:?}"
    );
}
