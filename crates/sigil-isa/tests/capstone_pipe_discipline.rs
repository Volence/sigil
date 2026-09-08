//! The pipe discipline of `capstone_diff::run_piped`, the child runner both
//! capstone gates go through.
//!
//! A parent that writes ALL of a child's stdin before reading any of its
//! stdout deadlocks as soon as both pipes are full: the child sleeps in a
//! stdout write waiting for a reader that is asleep in its stdin write. Such
//! a run does not fail, it HANGS, which defeats `--no-fail-fast`, defeats a
//! log aggregate, and defeats an agent polling its own log for an end marker.
//! The runner therefore writes stdin from a joined thread while the calling
//! thread drains stdout and stderr, and this file is what keeps that order:
//! every case here is driven by a child built to block a write-then-read
//! parent for certain, so a revert to that shape turns these cases red (by
//! watchdog, within the minute) instead of turning the suite into a hang.
//!
//! # How the volumes are derived
//!
//! A pipe's capacity is not a constant a test may copy: it is 64 KiB by
//! default, at most `fs.pipe-max-size`, and ONE PAGE once the user's pipe
//! pages exceed `fs.pipe-user-pages-soft`. So each case moves
//! `2 * pipe-max-size + 1` bytes in every direction it needs full, read from
//! `/proc` at run time. Whatever capacity a pipe was created with is at most
//! `pipe-max-size`, so that volume fills it with the same volume again to
//! spare, and a machine that cannot answer the question fails loudly rather
//! than measuring against a guess.
//!
//! # Why every case has a watchdog
//!
//! The failure this file exists to catch is a hang, so an unguarded case
//! would regress into exactly the thing it guards against. Each run of the
//! runner happens on a thread the case waits on with a limit; the limit is
//! [`WATCHDOG`], and a case that reaches it panics with the diagnosis. The
//! work itself is a few MiB through pipes (milliseconds), so the limit sits
//! more than two orders of magnitude above the measured time and a loaded
//! machine cannot reach it honestly, while a deadlock still reports within
//! it rather than never.

#[path = "support/capstone_diff.rs"]
mod capstone_diff;

use capstone_diff::{capstone_or_skip, run_piped, Cap, PAD_LEN};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// See the module docs: two orders of magnitude above the measured cost of
/// any case here, and the bound on how long a regression takes to report.
const WATCHDOG: Duration = Duration::from_secs(120);

/// `fs.pipe-max-size`, the largest capacity any pipe on this kernel can
/// have. Loud when it cannot be read: a volume derived from a guess would
/// let a smaller-than-guessed pipe make the deadlock cases vacuous.
fn pipe_max_size() -> usize {
    let raw = std::fs::read_to_string("/proc/sys/fs/pipe-max-size")
        .unwrap_or_else(|e| panic!("cannot read /proc/sys/fs/pipe-max-size ({e}), so the pipe volume that makes these cases decisive cannot be derived"));
    raw.trim()
        .parse()
        .unwrap_or_else(|e| panic!("/proc/sys/fs/pipe-max-size holds {raw:?}, not a byte count ({e})"))
}

/// A volume that fills a pipe of ANY capacity this kernel allows, with the
/// same again to spare.
fn past_any_pipe() -> usize {
    2 * pipe_max_size() + 1
}

/// Run `f` on its own thread and wait at most [`WATCHDOG`] for its result,
/// returning it with the wall-clock it took. Reaching the limit is the
/// deadlock verdict; a panic inside `f` is re-raised here after the thread's
/// own report.
fn within<T: Send + 'static>(what: &str, f: impl FnOnce() -> T + Send + 'static) -> (T, Duration) {
    let (tx, rx) = mpsc::channel();
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(WATCHDOG) {
        Ok(v) => (v, started.elapsed()),
        Err(mpsc::RecvTimeoutError::Timeout) => panic!(
            "{what}: no result after {WATCHDOG:?}. The runner is deadlocked: the parent is \
             blocked writing the child's stdin while the child is blocked writing its stdout, \
             which is the write-then-read order this gate forbids"
        ),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            panic!("{what}: the runner panicked (its report is above)")
        }
    }
}

/// A child that writes `n` bytes to stdout BEFORE it reads a byte of stdin,
/// then consumes all of stdin. A write-then-read parent cannot get past it.
fn stdout_first_child(n: usize) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(format!("head -c {n} /dev/zero && cat >/dev/null"));
    cmd
}

#[test]
fn a_child_that_fills_its_stdout_before_reading_stdin_completes() {
    let n = past_any_pipe();
    let cmd = stdout_first_child(n);
    let (res, took) = within("stdout-first child", move || run_piped("probe child", cmd, Some("x".repeat(n))));
    let out = res.unwrap_or_else(|e| panic!("the runner reported an error instead of completing: {e}"));
    println!("stdout-first child: {n} bytes each way in {took:?}");
    // Both pipes were full only if the child really produced the whole volume
    // and the parent's write really went through; a short child (no `head`,
    // say) would have measured nothing.
    assert_eq!(
        out.stdout.len(),
        n,
        "the child produced {} of {n} stdout bytes, so its stdout pipe was never full and this run measured nothing",
        out.stdout.len()
    );
}

#[test]
fn a_write_the_child_never_reads_surfaces_as_err() {
    let n = past_any_pipe();
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("exit 0");
    let (res, took) = within("child that ignores stdin", move || run_piped("probe child", cmd, Some("x".repeat(n))));
    println!("child that ignores stdin: {took:?}");
    let err = res.expect_err("a write the child never read cannot have succeeded, yet the runner reported Ok");
    assert!(
        err.contains("stdin"),
        "the write failure must name the stdin write, got: {err}"
    );
}

#[test]
fn a_failing_child_reports_its_stderr_not_the_broken_pipe() {
    let n = past_any_pipe();
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("echo no oracle here >&2; exit 3");
    let (res, took) = within("child that fails before reading", move || run_piped("probe child", cmd, Some("x".repeat(n))));
    println!("child that fails before reading: {took:?}");
    let err = res.expect_err("a child that exited 3 cannot have succeeded, yet the runner reported Ok");
    assert!(
        err.contains("no oracle here") && err.contains("exit status: 3"),
        "the child's own explanation must win over the write failure it caused, got: {err}"
    );
    assert!(
        !err.contains("Broken pipe"),
        "the broken pipe is the consequence, not the reason, and must not displace the reason: {err}"
    );
}

/// The shipped pair, dump script included, on an input past what the
/// emitted-stream corpus feeds it: the discipline holds end to end, not only
/// for the synthetic child above. The line count is derived so the input
/// alone fills both pipes of any capacity twice over, before the child's
/// 1.6x larger reply is counted.
#[test]
fn the_real_oracle_pair_survives_an_input_past_both_pipes() {
    let line_len = 2 * PAD_LEN + 1;
    let lines = 2 * past_any_pipe() / line_len + 1;
    let text: String = (0..lines)
        .map(|i| {
            let word = (i.wrapping_mul(2_654_435_761)) & 0xFFFF;
            format!("{word:04X}{}\n", "00".repeat(PAD_LEN - 2))
        })
        .collect();
    let bytes = text.len();
    let (recs, took) = within("real oracle pair", move || capstone_or_skip("bytes", &[], Some(text)));
    let Some(recs) = recs else { return };
    println!("real oracle pair: {lines} lines ({bytes} bytes) in {took:?}");
    assert_eq!(
        recs.len(),
        lines,
        "capstone answered for {} of {lines} lines, so part of the input never reached it",
        recs.len()
    );
    assert!(
        recs.iter().any(|(_, c)| matches!(c, Cap::Ok { .. })),
        "capstone decoded none of {lines} opcode words, the oracle is not answering"
    );
}
