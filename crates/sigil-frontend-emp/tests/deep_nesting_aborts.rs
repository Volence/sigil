//! Depth-guard regressions for the two confirmed process ABORTS (sigil lens
//! sweep 2026-08-13, seat SAFE, finding S19).
//!
//! These are not panics. A stack overflow raises SIGABRT, which cannot be
//! `catch_unwind`'d, so the process simply dies — no diagnostic, no location,
//! nothing for a caller to report. `sigil` must always fail with a message.
//!
//! Each case therefore runs the parser in a CHILD THREAD with a bounded stack and
//! asserts it returns. A child thread is used deliberately: it isolates the depth
//! behaviour from whatever stack the test harness happens to give the main thread,
//! so the assertion is about the guard and not about the runner.
use std::sync::mpsc;

/// Parse on a worker thread with a modest stack. Returns the diagnostics, or
/// panics if the parse did not finish.
fn parse_bounded(src: String) -> Vec<sigil_span::Diagnostic> {
    let (tx, rx) = mpsc::channel();
    let h = std::thread::Builder::new()
        .stack_size(4 * 1024 * 1024)
        .spawn(move || {
            let (_f, diags) = sigil_frontend_emp::parse_str(&src);
            let _ = tx.send(diags);
        })
        .expect("spawn");
    let out = rx
        .recv_timeout(std::time::Duration::from_secs(60))
        .expect("parse did not terminate");
    h.join().expect("parser thread died, stack overflow regression");
    out
}

/// `MAX_EXPR_DEPTH` is 128, so a guard that truly bounds recursion is
/// indifferent to input size. Before the fix, the bracket-index arm re-read the
/// `[` it never consumed and descended again per token, making depth track INPUT
/// SIZE: 30,000 aborted the process.
#[test]
fn index_chain_does_not_abort() {
    let n = 60_000;
    let src = format!("module m in s\n\nconst X = a{}1{}\n", "[".repeat(n), "]".repeat(n));
    let diags = parse_bounded(src);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "a {n}-deep index chain must be a clean error"
    );
}

/// The same shape in ARRAY-LITERAL position, which also aborted. Not named in the
/// packet — found while checking whether the index arm was the only offender.
#[test]
fn array_literal_nesting_does_not_abort() {
    let n = 60_000;
    let src = format!("module m in s\n\nconst X = {}1{}\n", "[".repeat(n), "]".repeat(n));
    let diags = parse_bounded(src);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "a {n}-deep array literal must be a clean error"
    );
}

/// The control that proves the mechanism: parenthesis nesting was ALREADY
/// correctly bounded (it descends only through the guarded `primary_expr`), and
/// must stay that way.
#[test]
fn paren_nesting_stays_bounded() {
    let n = 60_000;
    let src = format!("module m in s\n\nconst X = {}1{}\n", "(".repeat(n), ")".repeat(n));
    let diags = parse_bounded(src);
    assert!(diags.iter().any(|d| d.level == sigil_span::Level::Error));
}

/// The depth diagnostic is emitted once per latch, not once per re-descent. A
/// 60,000-token bomb used to emit hundreds of identical lines; the flood is the
/// visible symptom of the re-descent, so bounding it guards the same defect from
/// the other side.
#[test]
fn depth_diagnostic_does_not_flood() {
    let n = 60_000;
    let src = format!("module m in s\n\nconst X = {}1\n", "-".repeat(n));
    let diags = parse_bounded(src);
    let deep = diags.iter().filter(|d| d.message.contains("nesting too deep")).count();
    assert!(
        (1..=4).contains(&deep),
        "expected the depth limit to be reported a handful of times, got {deep}"
    );
}
