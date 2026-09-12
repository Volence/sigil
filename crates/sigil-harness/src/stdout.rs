//! Every stdout write a sigil binary makes, and what happens to it when the
//! reader of stdout has gone.
//!
//! A binary opts in with `use sigil_harness::stdout::{print, println};`. The
//! import shadows std's macros of the same names, so each `print!` and
//! `println!` in that file lands in [`write`] while the call sites keep the
//! spelling every reader, and every gate that reads source text, already knows.
//! `tests/stdout_writer_population.rs` holds the population to this: a binary
//! that writes stdout without the import, or any code that takes a stdout handle
//! of its own, fails there.
//!
//! # The broken-pipe rule
//!
//! Rust starts a process with SIGPIPE ignored, so a write to a pipe whose read
//! end has closed returns `EPIPE` rather than killing the process, and std's
//! `println!` panics on that error: exit 101, with `failed printing to stdout:
//! Broken pipe` on stderr. With std's macros, `sigil --version | head -1` ends
//! that way whenever `head` exits before the banner is written, and a script
//! under `pipefail` reads a sound assembler as a crashed one.
//!
//! Here, the first write that fails with `BrokenPipe` records that stdout's
//! reader has gone, and every later stdout write is dropped without a syscall.
//! The process is NOT ended there: it runs on to the end it would have reached
//! with a live reader and exits with that status, printing nothing about the
//! pipe.
//!
//! - Silent, because the reader asked for less output, which is not a failure
//!   of the tool.
//! - The run's own status, not a fixed 0, because the broken write can land on
//!   a failing run's report. `sigil parse` prints its diagnostics on stdout and
//!   exits 1, `sigil <root.asm>` prints its failure line on stdout; exiting 0 at
//!   the broken write would hand `pipefail` a success for both, and would also
//!   abandon a run that had an artifact still to write.
//! - Not the default SIGPIPE disposition, because the kernel then ends the
//!   process with status 141, which `pipefail` reads as a failure exactly as it
//!   reads 101.
//!
//! # What the rule does not cover
//!
//! Only `BrokenPipe`, and only on stdout. Any other stdout error panics with
//! std's own message, `failed printing to stdout: <error>`, so a full disk under
//! `sigil <root.asm> --hex > file` fails the run as std's `println!` fails it.
//! stderr is untouched: `eprint!` and `eprintln!` stay std's.
//!
//! One difference from std's macros that no binary's user can see: std's
//! `print!` writes into libtest's capture buffer when called from a unit test,
//! and [`write`] goes to the process's stdout. No test code in a binary prints.

use std::fmt;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// Set by the first stdout write that fails with `BrokenPipe`. Never cleared: a
/// pipe whose read end has closed does not reopen.
static READER_GONE: AtomicBool = AtomicBool::new(false);

/// Write `args` to stdout under the broken-pipe rule in the module note.
///
/// `#[track_caller]` so the panic on any other error names the print site in
/// the binary rather than this function.
#[track_caller]
pub fn write(args: fmt::Arguments<'_>) {
    if READER_GONE.load(Ordering::Relaxed) {
        return;
    }
    if let Err(e) = io::stdout().lock().write_fmt(args) {
        if e.kind() == io::ErrorKind::BrokenPipe {
            READER_GONE.store(true, Ordering::Relaxed);
        } else {
            panic!("failed printing to stdout: {e}");
        }
    }
}

/// std's `print!`, through [`write`]. Reached as `sigil_harness::stdout::print`.
#[doc(hidden)]
#[macro_export]
macro_rules! __sigil_stdout_print {
    ($($arg:tt)*) => {
        $crate::stdout::write(::std::format_args!($($arg)*))
    };
}

/// std's `println!`, through [`write`]: the formatted text and its newline in
/// one write, the bytes std's `println!` writes. Reached as
/// `sigil_harness::stdout::println`.
#[doc(hidden)]
#[macro_export]
macro_rules! __sigil_stdout_println {
    () => {
        $crate::stdout::write(::std::format_args!("\n"))
    };
    ($($arg:tt)*) => {
        $crate::stdout::write(::std::format_args!("{}\n", ::std::format_args!($($arg)*)))
    };
}

pub use __sigil_stdout_print as print;
pub use __sigil_stdout_println as println;
