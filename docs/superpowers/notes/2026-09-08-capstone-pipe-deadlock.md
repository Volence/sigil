# 2026-09-08: the capstone child runner, and a suite test that hung instead of failing

Closes ledger row `CAPSTONE-STREAM-PIPE-DEADLOCK`. Branch `parcel/capstone-pipe-deadlock` from
master `9df49aaf`; fix and gate at `d73c743d`. Reference tree `AEON_DIR=.aeon-ls12-fix` at aeon
`ec640bcf`, read-only.

## The defect

`run_capstone` (`crates/sigil-isa/tests/support/capstone_diff.rs`) wrote the whole of the dump
script's stdin with `write_all` and only then called `wait_with_output`. The moment both pipes are
full the parent sleeps in its write and the child (`scripts/capstone_m68k_dump.py`) sleeps in its
stdout write, each waiting for the other. `m68k_capstone_stream::every_emitted_m68k_instruction_agrees_with_capstone`
ran 1290 s that way in a full-suite run and would never have ended.

## Measured before changing anything

The parent side of the old discipline was reproduced exactly in a Python driver (write all, close,
then read) against the real dump script, with a hard kill at the timeout. Load average 12 to 17
throughout (other lanes' builds).

Real corpus, from the stream test at aeon `ec640bcf`: `distinct padded byte strings across all
shapes: 3257`, one 28-hex-digit line plus newline each, so **94,453 bytes of stdin**. The ledger
row's 325,830 is not the stdin volume; where that figure came from is not known.

Sweep against the default pipe (`F_GETPIPE_SZ` = 65536 here; python 3.14.7,
`io.DEFAULT_BUFFER_SIZE` = 131072):

```
n=  1000 in=   29000B out=   46265B ratio=1.60 elapsed=0.05s verdict=ok
n=  5000 in=  145000B out=  231198B ratio=1.59 elapsed=0.06s verdict=ok
n=  6000 in=  174000B out=       0B                elapsed=20.00s verdict=DEADLOCK(timeout 20s)
n= 11200 in=  324800B out=       0B                elapsed=20.00s verdict=DEADLOCK(timeout 20s)
n= 40000 in= 1160000B out=       0B                elapsed=20.00s verdict=DEADLOCK(timeout 20s)
```

The child answers 1.6 bytes per input byte. The threshold sits where the arithmetic puts it: the
child blocks once it has produced 128 KiB (its stdout buffer's first flush cannot fit a 64 KiB
pipe), having consumed 128 KiB / 1.6 = 80 KiB of input; the parent can have delivered that plus the
child's read-ahead (at most one pipe, 64 KiB) plus the stdin pipe (64 KiB), so 144 to 208 KiB of
input completes and more never does. **The real corpus, 94 KiB, is below the floor of that window
on a default pipe.** The scheduling race the row described cannot reach it.

Same driver with both pipes resized after spawn (`F_SETPIPE_SZ`):

```
pipe=  4096 n= 3257 in=  94453B out=      0B elapsed=15.00s verdict=DEADLOCK(timeout 15s)
pipe=  4096 n= 5000 in= 145000B out=      0B elapsed=15.00s verdict=DEADLOCK(timeout 15s)
pipe= 65536 n= 3257 in=  94453B out= 150651B elapsed=0.05s verdict=ok
pipe= 65536 n= 5000 in= 145000B out= 231198B elapsed=0.06s verdict=ok
```

One page is what the kernel hands a user whose pipe pages exceed `fs.pipe-user-pages-soft`
(16384 pages, 64 MiB, here). At one page the real corpus deadlocks every time. A full-suite run
holds hundreds of child pipes at once across cargo, rustc's jobserver and every test binary's
children, and several lanes were running suites at the time. So "load decides it" stands, but the
load is the user's pipe-page count, not CPU time, and the hanging run's pipe size was never
witnessed: this is the mechanism that fits every measured number, not an observed one.

## The fix

`run_piped(what, cmd, stdin)`: the stdin bytes go out from a spawned thread while the calling
thread drains stdout and stderr through `wait_with_output`; the writer is joined afterwards, so a
failed write is an `Err` and never a detached thread's private outcome. A non-zero child status is
reported ahead of the write error, deliberately: the old order turned a child that died before
reading (capstone import failure, exit 3) into `Broken pipe (os error 32)` and lost the child's
stderr whenever the input exceeded the pipe. Spawn failure, non-zero status and non-UTF-8 output
are `Err` exactly as before.

Rejected: a temp file for stdin. It removes the deadlock too, but it puts a filesystem write in
the path of every oracle call, needs a cleanup story, and still leaves stdout on a pipe that a
child could fill with stderr; the thread shape keeps the runner self-contained and generalises to
any child.

## The gate: `crates/sigil-isa/tests/capstone_pipe_discipline.rs`

Four cases, each on a thread under a 120 s watchdog (the measured cost is milliseconds to half a
second, so the limit is two orders of magnitude above honest completion, and a deadlock still
reports inside it rather than never):

- `a_child_that_fills_its_stdout_before_reading_stdin_completes`: `sh -c 'head -c N /dev/zero &&
  cat >/dev/null'` with N = 2 * `fs.pipe-max-size` + 1 bytes each way, read from `/proc` at run
  time and loud when unreadable. Any pipe capacity is at most pipe-max-size, so both pipes are full
  with the same again to spare, and the case asserts the child produced all N bytes so a short child
  cannot make it vacuous.
- `a_write_the_child_never_reads_surfaces_as_err`: `sh -c 'exit 0'`, N bytes of stdin, the error
  must name the stdin write.
- `a_failing_child_reports_its_stderr_not_the_broken_pipe`: `sh -c 'echo no oracle here >&2; exit
  3'`, the child's own message must win and `Broken pipe` must not appear.
- `the_real_oracle_pair_survives_an_input_past_both_pipes`: the shipped dump script on 144,632
  hex lines (4 MiB, 2 * (2 * pipe-max-size + 1) / 29 + 1) through `capstone_or_skip`, so it follows
  the repo's skip-or-strict-panic convention when capstone is absent.

### Red-first proof

Mutation applied to the FIXED runner (the writer thread replaced by the old sequential write),
shown on disk before the run:

```
$ git diff --stat
 crates/sigil-isa/tests/support/capstone_diff.rs | 14 +++++---------
 1 file changed, 5 insertions(+), 9 deletions(-)
133:    let writer: Option<std::thread::JoinHandle<std::io::Result<()>>> = None;
134-    if let Some(text) = stdin {
135-        use std::io::Write;
136-        child.stdin.take().expect("stdin was configured as a pipe").write_all(text.as_bytes()).map_err(|e| e.to_string())?;
137-    }
```

Red run, `timeout 600 cargo test --release -p sigil-isa --test capstone_pipe_discipline`, exit 101,
**120 s wall-clock to the watchdog**:

```
the write failure must name the stdin write, got: Broken pipe (os error 32)
test a_write_the_child_never_reads_surfaces_as_err ... FAILED
the child's own explanation must win over the write failure it caused, got: Broken pipe (os error 32)
test a_failing_child_reports_its_stderr_not_the_broken_pipe ... FAILED
stdout-first child: no result after 120s. The runner is deadlocked: ...
test a_child_that_fills_its_stdout_before_reading_stdin_completes ... FAILED
real oracle pair: no result after 120s. The runner is deadlocked: ...
test the_real_oracle_pair_survives_an_input_past_both_pipes ... FAILED
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 120.01s
```

Restored with `git checkout d73c743d -- crates/sigil-isa/tests/support/capstone_diff.rs`; md5 of
the restored file `a48eeffa8b0741fde80698228b2497ed` equals `git show d73c743d:<path> | md5sum`;
`git status` clean against HEAD. Green run after the restore: 4 passed, 0 failed, 0 ignored.

Note the two cases the mutation reds WITHOUT hanging: they show the old order also lost error
text, which is a second defect the row did not book.

## Every write-to-a-child's-stdin site in the repo

Search over the worktree at `9df49aaf`, excluding target directories. Alphabet, Rust:
`Stdio::piped`, `.stdin(`, `wait_with_output`, `child.stdin`, `Command::new`. Python and shell:
`subprocess`, `Popen`, `communicate`, `input=`, `stdin`.

The set of sites that write to a child's stdin from the parent:

- `crates/sigil-isa/tests/support/capstone_diff.rs`, `run_capstone` (the one fixed here; it is the
  only `.stdin(Stdio::piped())` in the workspace).

Sites that configure a child's stdin without writing to it: `crates/sigil-harness/src/rev_reachability.rs:298`
(`Stdio::null()`). Every other `Command::new` in the workspace (one hundred and some, almost all
`env!("CARGO_BIN_EXE_sigil")`, `git`, `bash`, `python3`) uses `.output()` or `.status()` with an
inherited or null stdin. The four Python `subprocess.run` sites (`golden/ab/suite_paths.py`,
`scripts/drift_report.py` twice, a probe note's `census.py`) pass no `input=` and no `stdin=`.
`scripts/capstone_m68k_dump.py` is the pipe's other end: it reads stdin line by line and writes as
it goes, which is correct for a child and is what a parent must accommodate.

## Verification (worktree HEAD `d73c743d`, clean, `SIGIL_STRICT_GATE=1`, `AEON_DIR` as above)

- `cargo test --release -p sigil-isa --test m68k_capstone_differential --test capstone_pipe_discipline`:
  capstone_pipe_discipline 4 passed, 0 failed, 0 ignored (stdout-first child 3.8 ms; real pair
  536.8 ms); m68k_capstone_differential 1 passed, 0 failed, 0 ignored. Exit 0.
- `cargo test --release -p sigil-harness --test m68k_capstone_stream`: 1 passed, 0 failed, 0 ignored,
  9.72 s; 3257 distinct byte strings, capstone decoded 3257 of 3257. Exit 0. (Baseline before the
  fix at `9df49aaf`, same tree: 1 passed in 10.37 s, the same 3257.)
- `cargo clippy --release --workspace --all-targets -- -D warnings`: exit 0, 9.02 s; the only
  warnings in the log are the C++ build script's `-Wmaybe-uninitialized` lines from
  `sigil-clownlzss-sys`, which are not rustc lints.

The full workspace suite was not run here; the landing gate runs it.
