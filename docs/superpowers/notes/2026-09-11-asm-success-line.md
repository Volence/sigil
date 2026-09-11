# The AS route reports success (2026-09-11)

Branch `parcel/asm-success-line`, base `6b72d513`.

## What changed

`sigil <root.asm>` said nothing on a clean run. Measured on the base binary
(md5 `c5a52f85a85e1b094b68deb83765fd92`): `sigil p.asm` and `sigil p.asm -o
out.bin` both exit 0 with empty stdout and empty stderr. A run that wrote a ROM,
a run that discarded the image, and a command that never started looked alike.
`fail_asm` already ended a failing run's stdout on a verdict; the success half
was missing.

`run_asm` now ends through `emit_image`, the output tail `sigil emp` already
used, so both routes print one line for one outcome. Verbatim, from the patched
binary on a three-byte program:

| run | stdout |
|---|---|
| `sigil p.asm -o out.bin` | `built: 3 bytes, wrote out.bin` |
| `sigil p.asm` | `built: 3 bytes, no file written (pass -o <path> to write one)` |
| `sigil p.asm --hex` | `01 02 03` then `built: 3 bytes, no file written (pass -o <path> to write one)` |
| `sigil p.asm --hex -o out.bin` | `01 02 03` then `built: 3 bytes, wrote out.bin` |
| `sigil m.asm --hex` (`message "hi"`, one byte) | `hi`, `01`, `built: 1 bytes, no file written (pass -o <path> to write one)` |

stderr stays empty on every clean run, and a run without `-o` stays exit 0.

`emit_image` now returns `Result<(), WriteFailed>` instead of calling
`process::exit(1)` on a failed write, because the AS route must end every
failure through `fail_asm` (`asm_failure_exits_all_go_through_fail_asm` reads
`run_asm`'s body and refuses a bare `process::exit(1)`). The two emp callers
exit 1 on `Err` exactly as before. `run_asm` keeps its six `fail_asm` sites and
its `Stage::` order, which the two structural gates count.

## Stream and `--hex`: the decision

**Stdout, and the `built:` line comes last, after the `--hex` line.** The
existing conventions decided it. Nothing here was a judgement call with a real
cost either way:

- **stderr is the diagnostic stream.** `scripts/corpus-baseline.sh` line-counts
  stderr into `NERR` and reads "exit 0 and `NERR` 0" as a clean assembly. A
  success line on stderr would count as one diagnostic on every clean run.
- **`fail_asm` puts the failure verdict on stdout, as the last line**
  (`asm_failure_line.rs`). The success verdict on the same stream, also last,
  means the last line of stdout says how the run ended either way, so a
  captured `> build.log` always ends on the verdict.
- **The emp route prints the same line, on stdout, after its `--hex` line.**
  Sharing the function makes that a property of one code path, not of two
  copies.

`--hex` stdout was never pure data on this route. The base binary prints
`hi\n01` for a `message "hi"` program under `--hex`, so a consumer already had
to read the hex LINE and not the whole stream. `as_message_stdout.rs` pins where
the `message` lines go, not a rule that nothing else may share stdout: its
failing-run test already asserts stdout carries more than the message.

## Consumer enumeration

### In this repo

| consumer | what it read | action |
|---|---|---|
| `crates/sigil-cli/tests/end_to_end.rs` (`--hex -o`) | all of stdout, trimmed, as the golden hex | reads line 0; asserts line 1 is the `built:` line |
| `cli_diagnostic_location.rs` `the_single_file_route_still_accepts_a_forward_reference` (`--hex`) | all of stdout, trimmed, as hex | first line |
| `undefined_jmp_jsr_target.rs` `a_forward_jsr_to_a_defined_label_still_assembles` (`--hex`) | all of stdout, trimmed, as hex | first line. Its `run` helper (also `--hex`) reads only exit code and stderr, so it is unaffected |
| `as_message_stdout.rs`, three success tests | all of stdout as the message text | `message_lines`: the messages exactly, with the `built:` line asserted last |
| `asm_failure_line.rs` `a_succeeding_run_says_nothing_about_failure` | stdout == the message | stdout is exactly [message, `built:` line]. The failing-run tests read the last line of a failure, which never reaches `emit_image`: unaffected |
| `partial_error_list_stage_note.rs` `a_succeeding_run_says_nothing_about_withheld_errors` | stdout == `""` | one `built:` line, no caveat. The structural gates still hold |
| `image_bounds.rs` | refused runs' stdout (failure line only); the accepted case reads the file | unaffected |
| `cli_help.rs` | a missing `.asm` file, which is a failure | unaffected |
| `scripts/z80_byte_sweep.sh` (`--hex 2>&1`) | all output, spaces and newlines stripped, as the byte string | `sed '/^built: /d'` before stripping. Checked in isolation: old-binary and new-binary output both give `70014E75`. Not run end to end: it needs the asl reference tree |
| `scripts/corpus-baseline.sh` (no `-o`, no `--hex`) | stdout only line-counted into `NOUT` and printed; the verdict keys on stderr's `NERR` | none. `NOUT` grows by one on a clean run and nothing compares it |
| `.f1probe/cmp.sh`, `.s1probe/2026-09-04/probe/cmp.sh`, `.s1probe/construct_probe.sh` | merged output shown to a person (`sed -n 1,60p`, `tr`) | none; nothing parses it |
| `sigil-harness` | does not spawn the `sigil` binary (its `Command::new` calls are git, bash, cargo, convsym, refreeze, repin) | none |
| emp-route tests reading `built:` (`dac_bank_acceptance`, `pitcher_plant*`, `const_arity_cli`, `emp_output_disposition`, `subcommands`) | the emp route's line | unchanged output |

### In aeon (origin/master `9c45616d`, read only through `git grep`)

Nothing in aeon runs the bare `sigil <file.asm>` route, so no external
consumer breaks. Searched: identifier forms (`SIGIL_BUILD`, `SIGIL_EMIT`,
`$SIGIL`, `sigil_bin`), quoted-string forms (`"sigil"`, `'sigil'`,
`/release/sigil`, `bin/sigil`), and `sigil ... .asm`. Every invocation found:

- `build.sh`: `"${SIGIL_BUILD}" --version`, and `sigil build ...`;
- `SIGIL_EMIT` is the separate `emit_sound_blob` binary (`build.sh:568`);
- `tools/emp_expect_fail.py:705`, `tools/extern_guard_census.py:260`,
  `tools/test_extern_guard_reachability.py:200`: `[SIGIL, "build", ...]`;
- `tools/drift_record.py:360`: `--version`;
- `tools/tick_variance_probe.py:199` sends its child's stdout to `DEVNULL`,
  so it reads nothing whatever it runs.

## Tests and the red-first proof

New file `crates/sigil-cli/tests/asm_output_disposition.rs`:

- `built_line_distinguishes_written_from_discarded`
- `hex_line_precedes_the_built_line_which_ends_stdout`
- `a_clean_run_reports_on_stdout_and_leaves_stderr_empty`
- `the_as_and_emp_routes_print_one_line_for_one_outcome`
- `a_failed_write_ends_through_fail_asm_and_reports_no_build`

Method, per mutation: on the committed tip `77ed1255` with a clean tree,
`mutate.py` applies the mutation and refuses unless its target occurs exactly
once. The mutated lines are quoted from `git diff -U0` of the on-disk file
before the run. Only this test binary runs, and `main.rs` is restored with
`git show HEAD:crates/sigil-cli/src/main.rs > main.rs`. Every restore was
checked: on-disk md5 `607f64122a80374330abf5f861ce102a` equals the HEAD blob,
and `git status` is clean.

| mutation | line quoted from disk | red | green (and why) |
|---|---|---|---|
| `base`: `main.rs` at `6b72d513` | `if let Some(out_path) = output {` / `if let Err(err) = install_artifact(&out_path, &image) {` (the pre-fix tail) | distinguishes, hex, clean-run, cross-route | failed-write: the base route already ended a write failure through `fail_asm`. That gate exists for the refactor (last row) |
| `same-line` | `None => println!("built: {} bytes", image.len()),` | distinguishes, hex | clean-run (stream unchanged), cross-route (both routes mutated alike), failed-write |
| `stderr` | `Some(out_path) => eprintln!("built: {} bytes, wrote {out_path}", image.len()),` and `None => eprintln!(` | distinguishes, hex, clean-run, cross-route | failed-write |
| `order`: hex block moved after the `match` | `+    if hex {` added after the `match`, removed before it | hex | the other four do not read the order |
| `ignore-write-failure` | `let _ = emit_image(&image, output.as_deref(), hex);` | failed-write | the other four never fail a write |

Every gate is red under at least one mutation, and every run turned at least
one gate red, so each run executed the file that was patched.

**Clippy positive control.** The first clippy run exited 0, but its log never
printed a `Checking sigil-cli` line, only `Compiling sigil-cli` (the build
script), so it did not show that the crate under test was linted. The same
harness planted `fn _clippy_probe() -> i32 { return 1; }` in `main.rs` (quoted
from disk as above), and the identical command `cargo clippy --release -p
sigil-cli --all-targets -- -D warnings` then exited 101 with `error: unneeded
`return` statement --> crates/sigil-cli/src/main.rs:865:5` and `could not
compile sigil-cli (bin "sigil")`. So clippy does lint this binary, and a 0 from
that command is a real pass. The restore was checked the same way.

## Suite and clippy

This was a deliberately partial run, with no reference tree named and no
`AEON_DIR` set: `SIGIL_ALLOW_PARTIAL=1 cargo test --release -p sigil-cli
--no-fail-fast` in this worktree, with HEAD `77ed1255` before and after.

| run | exit | binaries | passed | failed | ignored |
|---|---|---|---|---|---|
| captured (the brief's command) | 0 | 165 | 771 | 0 | 1 |
| the same plus `-- --nocapture` | 0 | 165 | 771 | 0 | 1 |

**How much went unmeasured.** The captured run prints NOTHING about it. The
partial-run banner and the `skip:` lines go to stderr from inside test threads,
and libtest captures a passing test's output, so a grep of that log for the
banner returns zero. That zero means the output was captured, not that nothing
was skipped. The uncaptured rerun shows them: 104 processes printed the banner
(once per process), each reading `PARTIAL RUN (SIGIL_ALLOW_PARTIAL is set). No
reference tree is named, so 130 test binaries are reference-dependent and every
row in them is left UNMEASURED.`, and 384 `skip:` lines printed. The banner's
130 comes from `reference_dependent_binaries(&workspace_root())`, a count over
the whole workspace, so for this `-p sigil-cli` run it is the workspace figure
rather than sigil-cli's own. The strict landing gate, with a reference tree
named, is the run that measures those rows.

`cargo clippy --release -p sigil-cli --all-targets -- -D warnings`: exit 0 on
the committed tip. The 19 lines in its log that match `warning:` are all
`warning: sigil-clownlzss-sys@0.1.0:` lines from the vendored C++ build script
(19 of 19). The positive control above shows that this command lints the
binary.

`the_published_line_states_this_revision_s_position_against_a_named_remote_ref`
and `version_reports_the_head_of_the_tree_it_was_built_from` both passed, with
no commit made during either run.

## Open

- The shared line says `built: 1 bytes` for a one-byte image. That wording is
  the emp route's too, and it is flagged here rather than changed.
- `scripts/z80_byte_sweep.sh` was checked only in isolation (see the table).
- The parcel's base is `6b72d513`, the worktree's HEAD. Master stood at
  `0907078a`, four commits ahead, and none of those four touch
  `crates/sigil-cli` or `scripts`.
