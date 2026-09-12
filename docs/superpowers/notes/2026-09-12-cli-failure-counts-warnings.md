# CLI-FAILURE-COUNTS-WARNINGS: the AS failure line counted warnings as errors

2026-09-12. Branch `parcel/cli-failure-counts-warnings`, fix commit `326c41bb`. Queue row
`CLI-FAILURE-COUNTS-WARNINGS`, from the "Open, and why" list of
`2026-09-11-s2-as-small-features.md`.

## The defect, reproduced before any change

Probe `2026-09-11-s2-as-small-features/probes/cli_warn_count.asm` (one author `warning`, one
unknown mnemonic). Binary `sigil 0.1.0 (dec6dcb0)`, tree clean at capture, built into this
lane's own target directory:

```
$ sigil cli_warn_count.asm
EXIT=1
--- stdout
this error list may be incomplete: sigil stopped at the front end, so layout, link and the image checks did not run
assembly failed: 2 errors (reported on stderr)
--- stderr
cli_warn_count.asm(4):2: warning: [as.warning] author
cli_warn_count.asm(5):2: error: `zzbogus` is not a recognized 68000 mnemonic
```

One error and one warning on stderr, `2 errors` on stdout.

## What asl says about the same probes

Reference build `s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098`,
run through `asl_ref.sh`'s `asl_run`. Three of the four probes are failing runs (ASL_EXIT=2), so
under OVERSEER-REFERENCE's third face nothing here is quoted as a VALUE; what is read is the
accept or refuse verdict and the footer's diagnostic counts, which are the subject.

| probe | asl exit | asl footer | sigil before | sigil after |
|---|---|---|---|---|
| `cli_warn_count.asm` (1 error, 1 warning) | 2 | `1 error` / `1 warning` | `assembly failed: 2 errors` | `assembly failed: 1 error, 1 warning` |
| `e1w2.asm` (1 error, 2 warnings) | 2 | `1 error` / `2 warnings` | (3 errors, per mutation M1 below) | `assembly failed: 1 error, 2 warnings` |
| `e2w1.asm` (2 errors, 1 warning) | 2 | `2 errors` / `1 warning` | (3 errors) | `assembly failed: 2 errors, 1 warning` |
| `w1only.asm` (1 warning) | 0 | `0 errors` / `1 warning` | exit 0, `built:` | exit 0, `built:` (unchanged) |

(Every "after" line ends ` (reported on stderr)`.) asl never folds the two: its footer prints
them as two lines, zeros included. The "before" cells for the two new probes are not a run of
the old binary (it was relinked); they are what mutation M1, which restores the old counting,
printed for the same shape.

## The population: every list that reaches `fail_asm`

| call site in `run_asm` | list | levels it can hold |
|---|---|---|
| front end `Err(failure)` | `failure.diags` | Error and Warning. Every `Failure` holds at least one Error: `eval.rs` pushes one before the returns at 361, 552 and 592 (the last two also merge the carried author warnings in); the returns at 479 and 525 are guarded by `any(Error)`; `lib.rs:252` (root unreadable) is an Error. |
| `resolve_layout_placing` | `diags` | Error only (`relax.rs`, every site `Level::Error`) |
| `link` | `diags` | Error only (`lib.rs` `diag()` helper) |
| `check_image_bounds` | `bounds` | Error only (`lib.rs:929`) |
| `flatten_placing` | `diags` | Error only (`blob.rs:286`) |
| `flatten` | a `String` | one error, printed `error: <msg>` |
| `emit_image` write failure | none returned | one error, printed `error: cannot write ...` |

Every warning the front end raises is a `Level::Warning`: the author `warning` directive
(`[as.warning]`), the `shared` directive's warning that the originating note's bullet names
(`directive_shared`, `eval.rs:10685`), and the unpopped value-stack warning. The fix counts by
level, so all three are counted as warnings whichever one a probe uses.

The one non-literal level in `sigil-link` (`check_link_asserts`, `lib.rs:436`, the Warning-tier
`[layout.odd-item]` assert) is called only from the `.emp` route (`main.rs` `link_to_image`),
never from `run_asm`. `Level::Note` is constructed nowhere in `sigil-frontend-as` or
`sigil-link`.

**So the brief's hypothesis was half the population.** Only the front-end list carries
warnings. But a second set of rendered warnings reached no list at all: on a run whose front end
SUCCEEDS, `render_as_warnings` prints the front end's warnings, and a failure at layout, link or
the image checks then counted only that stage's list. Fixing the first half alone would have made
a front-end failure read `1 error, 2 warnings` and a link failure over the same warnings read
`1 error`. Both are fixed: the tally starts from the front end's warnings and every later stage
adds to it.

**A zero error count is unreachable** from every caller today (the table above). The line still
has a defined answer for it, see below.

## The fix

`fail_asm(errors: usize, ..)` became `fail_asm(shown: Shown, ..)`. `Shown { errors, warnings,
notes }` is filled by `Shown::plus(&[Diagnostic])`, an exhaustive `match` on each diagnostic's
level (a new `Level` variant stops it compiling rather than landing in a wildcard arm), carried
from the front end's warnings through every stage. The `flatten` `String` is made an
`unlocated_error` diagnostic and rendered through `render_located_diags`, which prints the same
`error: <msg>` text, so it too is counted from the list it is printed from. `emit_image` hands
back no diagnostic, so its site adds one printed error through `Shown::plus_printed_error`.

Wording, from `failure_line`:

- errors only: `assembly failed: 1 error (reported on stderr)`, byte-identical to before;
- with warnings: `assembly failed: 1 error, 2 warnings (reported on stderr)`;
- with notes (unreachable on this route today, classified rather than folded): `, N notes`;
- zero errors: `assembly failed: 0 errors, 2 warnings (reported on stderr); a failure with no
  error is a sigil defect, please report it`.

Why: asl keeps the severities apart and prints both, so the line names both. A zero warning
count is omitted rather than printed as `0 warnings`, following sigil's own `warning_summary`,
which omits an empty bucket; that also keeps the common warning-free spelling byte-identical, so
the six tests that pin it and every frozen transcript under `docs/` still describe current
behaviour. The error count is always printed, zero included, because the line's first job is to
say what failed the build; a zero there is a defect in sigil, not in the program, and saying so
stops a reader concluding that the warnings it names were promoted to errors. The incompleteness
caveat still prints first and the failure line is still the last line of stdout.

## Siblings: every other failure-summary line the CLI prints

| line | where | verdict |
|---|---|---|
| `.emp` routes (`sigil emp`, `emp --root`, `sigil build`, `check`, `--report ram`, contract gate) | `run_emp`, `run_emp_program`, `run_build_native`, `run_check_native`, `run_ram_report`, `scan_or_exit`, `corpus_closure_or_exit`, `run_contract_gate` | print no count line at all; they render diagnostics and exit 1. Not in the class. |
| `test result: ok. N passed; M failed` | `run_test` | counts TESTS, not diagnostics. Not in the class. |
| `warning: N warnings, M notes, <id> <n>, ...` | `warning_summary` | already splits warnings from notes; carries no error count. Not in the class. |
| `{what}: N error(s):` | `sigil-harness/src/diag_render.rs:87` | filters to `Level::Error` before counting. Correct. |
| `manifest scan: N error(s)` | `sigil-harness/src/native.rs:1939` | filters to `Level::Error`. Correct. |
| front end's warnings uncounted after a later-stage failure | `run_asm` | same class, found here, fixed in `326c41bb`. |

## Consumers of the line, enumerated before the change

Grepped with `git grep` (tracked files only, `.claude/worktrees` excluded) in sigil, aeon,
empyrean, oracle, aurora, seraph and dominion, for the literal `assembly failed`, for
`reported on stderr`, and for a count pattern `[0-9]+ errors? \(`.

| repo | hit | reads it how |
|---|---|---|
| sigil | `crates/sigil-cli/src/main.rs` | the producer |
| sigil | `crates/sigil-cli/tests/asm_failure_line.rs` (4 sites), `as_message_stdout.rs:104`, `asm_output_disposition.rs:296`, `image_bounds.rs:82`, `partial_error_list_stage_note.rs:287,290` | BRANCHES (exact-text asserts). Every probe is warning-free, so the spelling they pin is unchanged; all green. |
| sigil | `scripts/corpus-baseline.sh:160-166` | PASSES stdout to `$LABEL.out` and line-counts it (`NOUT`), then DISPLAYS the count. Never reads the text. `NERR`, the class table, the baseline `comm` and the clean verdict all read stderr. The fix adds words to an existing line, so `NOUT` is unchanged. |
| sigil | 25 files under `docs/` (notes, lens findings, and frozen `.log`/`.stdout`/`.out` transcripts) | DISPLAY. Records of past runs; not rewritten. |
| empyrean | `docs/SIGIL_AEON_COMPAT_NOTES.md:26`; `docs/records/plans/2026-07-01-sigil-core-foundation.md:1895` | DISPLAY. Prose, and a historical plan's `error: assembly failed: {:?}`, a different, older string. |
| aurora | `docs/lane-log.jsonl:99`, `docs/reviews/...`, a spec line | unrelated matches of the count pattern and of the phrase ("the mapping assembly failed to load"). |
| aeon | count-pattern hits in plan docs (pytest `Expected: 4 errors`) | unrelated. No hit for the literal. |
| oracle, seraph, dominion | none | |

No consumer outside sigil parses the line or its count, so the wording change was not BLOCKED.

## Tests, and each one seen red

Added: `asm_failure_line.rs`
`a_front_end_failure_counts_its_warnings_apart_from_its_errors` (1 error, 2 warnings, front end)
and `a_later_stage_failure_counts_the_front_ends_warnings` (the same two warnings, then a link
refusal); `main.rs` unit tests `shown_counts_each_severity_under_its_own_name` (1 error, 2
warnings, 3 notes) and `failure_line_names_each_severity_and_flags_a_failure_with_no_error`.
Expected values are literals. Each behavioural test first asserts its probe rendered exactly 1
error and 2 warnings on stderr and names the stage it stopped at, so the literal it then checks
is about the population it claims. Runners: `cargo test -p sigil-cli --test asm_failure_line`
and `--bin sigil`, both in the workspace suite.

Each mutation was applied to the committed baseline `326c41bb`, shown on disk, run, and restored
with `git restore --source=HEAD` after `git status --short` showed the mutated file as the only
change; `git status` was empty after each restore.

| mutation (on disk) | red | failing assertion text |
|---|---|---|
| M1, `main.rs:666` `sigil_span::Level::Warning => self.errors += 1,` (the original defect) | `shown_counts_...`, `a_front_end_...`, `a_later_stage_...` | `left: Shown { errors: 3, warnings: 0, notes: 3 }` / `right: Shown { errors: 1, warnings: 2, notes: 3 }`; and at `asm_failure_line.rs:152` and `:171`, `left: "assembly failed: 3 errors (reported on stderr)"` / `right: "assembly failed: 1 error, 2 warnings (reported on stderr)"` |
| M2, `main.rs:435` `let shown = Shown::default();` (front-end warnings not carried) | `a_later_stage_...` only | `asm_failure_line.rs:171`: `left: "assembly failed: 1 error (reported on stderr)"` / `right: "... 1 error, 2 warnings ..."` |
| M3, `main.rs:693,695` counts swapped in `failure_line` | the unit wording test and five `asm_failure_line` tests | `:152`: `left: "assembly failed: 2 errors, 1 warning (reported on stderr)"` / `right: "assembly failed: 1 error, 2 warnings (reported on stderr)"` |
| M4, `main.rs:701` `if false {` (zero-error clause dropped) | `failure_line_...` only | `left: "assembly failed: 0 errors, 2 warnings (reported on stderr)"` / `right: "...; a failure with no error is a sigil defect, please report it"` |

Every failing assertion is the count or wording assertion; none is a probe-shape guard. The
logs are `m1.log` to `m4.log` in the lane's scratch directory (not committed).

## Suite

Measured at HEAD `326c41bb7a30ba5839cf01a9d9259e0cc616e2a3` (the fix commit), branch
`parcel/cli-failure-counts-warnings`, tree clean (`git status --short` empty), from 06:28:35Z to
06:34:55Z on 2026-09-12. Command:
`CARGO_TARGET_DIR=<lane target> AEON_DIR=/home/volence/sonic_hacks/.aeon-sigil-ref-cli
SIGIL_STRICT_GATE=1 cargo test --release --workspace --no-fail-fast`. The aeon tree was
provisioned by `scripts/provision-aeon-ref.sh` (both rebuild controls MATCHES THE GOLDEN) and
witnessed by `repin --check`: exit 0, `pins.rs unchanged`.

**477 test binaries: 5316 passed, 1 failed, 2 ignored.** `SUITE_END rc=101`.

The one failure: `sigil-harness --test m1b_gate`,
`oracle_loadfromaslisting_resolves_emit_listing`, panicking at
`crates/sigil-harness/src/test_support.rs:1346` with `NO REFERENCE TREE IS NAMED, so this run can
measure nothing it could attribute, and STOPS. This run DECLINED to use
/home/volence/sonic_hacks/oracle-old, which step 3 derived from this checkout's own location`.
The command names `AEON_DIR` and no `ORACLE_DIR`, and under the strict gate this test declines a
derived path. Re-run of that one binary at the same HEAD with
`ORACLE_DIR=/home/volence/sonic_hacks/oracle-old` named: 5 passed, 0 failed, exit 0. It reads the
legacy emulator's symbol loader and has no path through `sigil-cli`.

`cargo clippy --release --workspace --all-targets -- -D warnings`: `CLIPPY_END rc=0`. The 19
`warning:` lines in its log are all the C compiler's, from `sigil-clownlzss-sys`'s build script,
not clippy lints.

This note's commit changes no compiled path, so the totals above are the tip's too.

## Open

- Nothing in the class is left open.
- The `.emp` routes print nothing on stdout when they fail, so `sigil emp x.emp > log` leaves an
  empty log. That is the C-compiler case `fail_asm`'s own comment calls harmless (an empty log
  misleads nobody), not this defect; noted, not changed.

## Things in the brief that turned out wrong

1. **"Those lists carry every rendered diagnostic whatever its severity."** Only the front end's
   list carries a warning; the layout, link and image lists are errors only. And the population
   was larger than the lists: the front end's warnings from a SUCCESSFUL pass were rendered and
   reached no list, so a later-stage failure would still have miscounted after a list-only fix.
2. **The full-suite command cannot go green as written.** It names `AEON_DIR` and not
   `ORACLE_DIR` (or `EMPYREAN_SUITE_ROOT`), and under `SIGIL_STRICT_GATE=1` the `m1b_gate`
   oracle test refuses a derived reference tree and fails. Named, it passes.
3. Nothing else. `fail_asm`, its callers and their line numbers, the probe's location, and
   `corpus-baseline.sh` reading stderr (it captures and line-counts stdout, and parses only
   stderr) all checked out as stated.
