//! PHASE TIMING for the AS route: where a run's wall time and memory go.
//!
//! With `SIGIL_PHASE_TIMING` set in the environment, the AS front end and `sigil
//! <file.asm>` print one line per phase to stderr:
//!
//! ```text
//! SIGIL-PHASE <tab> <name> <tab> wall_ms=<ms> <tab> hwm_kb=<kb> <tab> rss_kb=<kb> [<tab> key=value ...]
//! ```
//!
//! `wall_ms` is the phase's own wall time. `rss_kb` is the resident set at the end of the
//! phase. `hwm_kb` is the peak resident set since the last TOP-LEVEL phase ended: a
//! top-level phase ([`phase`]) restarts the kernel's peak counter after printing, so its
//! `hwm_kb` is its own peak; a step inside one ([`step`]) does not, so a step's `hwm_kb`
//! is the peak of its enclosing phase so far, and the step where it rises is where the
//! peak was reached. The allocator keeps freed memory resident, so a phase's peak starts
//! from whatever the phases before it left resident, not from zero.
//!
//! Accumulators ([`Acc`]) total work that happens in many small pieces inside a phase
//! (every `include` read, every line lexed); [`drain`] reads and zeroes them.
//!
//! Unset, every entry point returns after one load of a `OnceLock` and takes no clock
//! reading, so a normal build pays nothing measurable.
//!
//! read-set: not a build input. The one file this module reads is `/proc/self/status`,
//! the process's own memory counters, and the one it writes is `/proc/self/clear_refs`,
//! to restart the peak counter; no build output depends on either.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

/// The environment variable that turns phase timing on.
pub const VAR: &str = "SIGIL_PHASE_TIMING";

/// The prefix every line this module prints begins with.
pub const PREFIX: &str = "SIGIL-PHASE";

static ON: OnceLock<Option<Instant>> = OnceLock::new();

/// Whether phase timing is on. The first call reads the environment and, when the
/// variable is set, records the instant [`total`] measures from.
pub fn on() -> bool {
    ON.get_or_init(|| std::env::var_os(VAR).is_some().then(Instant::now)).is_some()
}

/// The start of a timed region: `Some(now)` when timing is on, `None` otherwise.
#[inline]
pub fn clock() -> Option<Instant> {
    if on() {
        Some(Instant::now())
    } else {
        None
    }
}

/// Work totalled across many small pieces.
#[derive(Copy, Clone, Debug)]
pub enum Acc {
    /// Reading an `include`d file's text (the read-set recorder included).
    IncludeRead,
    /// Splitting a source file's text into logical lines.
    SplitLines,
    /// Reading a `BINCLUDE`d file's bytes (the read-set recorder included).
    BincludeRead,
    /// Lexing one line into tokens.
    Lex,
}

const ACCS: [(Acc, &str); 4] = [
    (Acc::IncludeRead, "include_read"),
    (Acc::SplitLines, "split_lines"),
    (Acc::BincludeRead, "binclude_read"),
    (Acc::Lex, "lex"),
];

struct Tally {
    nanos: AtomicU64,
    calls: AtomicU64,
    bytes: AtomicU64,
}

static TALLIES: [Tally; 4] = [const { Tally { nanos: AtomicU64::new(0), calls: AtomicU64::new(0), bytes: AtomicU64::new(0) } }; 4];

/// Add the time since `t0`, one call and `bytes` to `acc`. Nothing when `t0` is `None`.
#[inline]
pub fn accumulate(acc: Acc, t0: Option<Instant>, bytes: u64) {
    if let Some(t0) = t0 {
        let t = &TALLIES[acc as usize];
        t.nanos.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
        t.calls.fetch_add(1, Ordering::Relaxed);
        t.bytes.fetch_add(bytes, Ordering::Relaxed);
    }
}

/// Every accumulator as `name_ms=.. name_calls=.. name_bytes=..` fields, each zeroed as
/// it is read. Empty when timing is off.
pub fn drain() -> String {
    if !on() {
        return String::new();
    }
    let mut out = String::new();
    for (acc, name) in ACCS {
        let t = &TALLIES[acc as usize];
        let nanos = t.nanos.swap(0, Ordering::Relaxed);
        let calls = t.calls.swap(0, Ordering::Relaxed);
        let bytes = t.bytes.swap(0, Ordering::Relaxed);
        if !out.is_empty() {
            out.push('\t');
        }
        out.push_str(&format!(
            "{name}_ms={:.3}\t{name}_calls={calls}\t{name}_bytes={bytes}",
            nanos as f64 / 1e6
        ));
    }
    out
}

/// The process's peak and current resident set, in kB, from `/proc/self/status`.
/// Zero for a field the kernel does not report.
fn memory() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |key: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(key))
            .and_then(|v| v.split_whitespace().next())
            .and_then(|n| n.parse().ok())
            .unwrap_or(0)
    };
    (field("VmHWM:"), field("VmRSS:"))
}

fn print(name: &str, t0: Instant, extra: &str) {
    let ms = t0.elapsed().as_secs_f64() * 1e3;
    let (hwm, rss) = memory();
    if extra.is_empty() {
        eprintln!("{PREFIX}\t{name}\twall_ms={ms:.3}\thwm_kb={hwm}\trss_kb={rss}");
    } else {
        eprintln!("{PREFIX}\t{name}\twall_ms={ms:.3}\thwm_kb={hwm}\trss_kb={rss}\t{extra}");
    }
}

/// End a TOP-LEVEL phase begun at `t0`: print its line, then restart the kernel's peak
/// counter so the next top-level phase's `hwm_kb` is its own. Nothing when `t0` is `None`.
pub fn phase(name: &str, t0: Option<Instant>, extra: &str) {
    if let Some(t0) = t0 {
        print(name, t0, extra);
        // "5" resets the peak resident set to the current one (proc(5), clear_refs).
        // A kernel that refuses leaves the peak monotonic, which the lines still read
        // correctly as a running maximum.
        let _ = std::fs::write("/proc/self/clear_refs", "5");
    }
}

/// End a step INSIDE a phase begun at `t0`: print its line without restarting the peak
/// counter. Nothing when `t0` is `None`.
pub fn step(name: &str, t0: Option<Instant>, extra: &str) {
    if let Some(t0) = t0 {
        print(name, t0, extra);
    }
}

/// Print the wall time since timing was switched on, as the phase `total`.
pub fn total() {
    if let Some(Some(origin)) = ON.get() {
        print("total", *origin, "");
    }
}
