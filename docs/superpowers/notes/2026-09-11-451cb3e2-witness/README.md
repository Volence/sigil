# The witness `451cb3e2` cites

Commit `451cb3e2` ("a failing run reports what an ordinary pass found, not the
deferral pass's quieter set") says the `Ok` to `Err` flip its gate can cause is
reachable, and names its witness as `.scratch/repro/control.asm`. That path is
in an untracked, per-worktree scratch area, so nobody else can follow it. This
directory is that witness, committed, with the two files that sat beside it.

## Where it was found

On 2026-09-11, at
`/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-af9ee78f0abdc942f/.scratch/repro/`,
a worktree whose `HEAD` names `refs/heads/parcel/failing-build-reports-everything`.

| file | md5 | mtime |
|---|---|---|
| `control.asm` | `6cfc6babe1fb3865b3432f4d72a8c988` | 2026-09-09 14:06 |
| `gated.asm` | `60d5b97f536c13103b1d3bc898c14879` | 2026-09-09 14:06 |
| `levartptrs.asm` | `4d67c00af4a4c6f38633ce545eced93d` | 2026-09-09 14:09 |

`451cb3e2`'s author date is 2026-09-09 14:11. The same scratch directory holds
`msg1.txt`, and it is `451cb3e2`'s commit message: `diff` against
`git show -s --format=%B 451cb3e2` differs only in the blank line git appends.
So these are the files the commit was written beside, not a reconstruction.
They are committed here byte for byte.

`docs/superpowers/notes/2026-09-10-ux-partial-error-list-repro/README.md`, and
the ledger row `sig-ux-partial-error-list` that cites it, say this file does not
exist because the worktree is gone. The worktree was still on disk on
2026-09-11 with the file in it. That directory's reconstruction answers a
different question (which stage boundaries withhold diagnostics), so it stands
on its own; only its statement that the witness is gone is wrong.

## What each file witnesses

- **`control.asm`** is the flip. A `dc.l` whose fold is `Poison` and whose
  operand names a label (`(NoSuchPlc<<24)|Lbl`), a `jsr` to a target defined
  nowhere, and nothing else wrong. Before `451cb3e2` the front end ran the
  deferral ("bonus") pass: `keep_labels_symbolic` short-circuits the `dc.l`
  operand before its fold, and the `jsr` becomes a deferred `JmpJsrSym`, so the
  front end returned `Ok` and layout refused the `jsr`. After it, the converged
  ordinary pass already holds the `dc.l`'s `unresolved long expression`, so the
  gate `!force_relocate && (poison.is_empty() || already_failed)` returns that
  pass, reports its leftover poison, and the front end returns `Err`.
- **`gated.asm`** is `control.asm` plus an unrelated error (`moveq #$1FF,d0`),
  so the run is failing for a reason the bonus pass has nothing to do with.
  It shows what the gate adds to a run that was going to fail anyway.
- **`levartptrs.asm`** is `sonic_hack`'s `levartptrs` macro reduced to one
  file, the same construct as the `LEVARTPTRS_FAILING` constant in
  `crates/sigil-frontend-as/tests/failed_run_reports_everything.rs`. The two
  texts differ in one line: the file indents the macro's `endm` by four
  spaces, and the constant carries it at column 0, because Rust's `\` line
  continuation drops the next line's leading whitespace. Both draw the same
  four diagnostics on the same lines, the test by assertion and the file in
  the table below.

## Measured 2026-09-11

Two `sigil` binaries, each built with `CARGO_TARGET_DIR` outside every checkout:

| binary | built from | md5 |
|---|---|---|
| before | `27db72a7`, the parent of `451cb3e2`, via `git archive` (so it reports `revision-unknown`: an archive carries no `.git`) | `cfaac2bd65eaf1b330a0d9949cfd20b9` |
| after | `5e3d389a` | `2a58bd6dc1311de598bae1cfefe7996c` |

`sigil <file> --hex`, run in this directory:

| file | before (`27db72a7`) | after (`5e3d389a`) |
|---|---|---|
| `control.asm` | exit 1, **1 error, from layout**: `(5) unresolved jmp/jsr target in section sec0 references symbol NoSuchTarget not defined in this link` | exit 1, **2 errors, from the front end**: `(4) unresolved long expression`, `(5) unresolved symbol NoSuchTarget in operand` |
| `gated.asm` | exit 1, 1 error: `(8)` the `moveq` | exit 1, 3 errors: `(4)` unresolved long expression, `(8)` the `moveq`, `(5)` `NoSuchTarget` |
| `levartptrs.asm` | exit 1, 1 error: `(23)` the `moveq` | exit 1, 4 errors: `(7)` and `(8)` unresolved long expression, `(23)` the `moveq`, `(21)` `LoadLevelLayout` |

The `control.asm` row is what the commit claims: the run still fails, the
refusal moves from a later stage to the front end, and it names both lines
instead of one. The commit's word for the later stage is "the linker"; in this
binary it is layout (`sigil_link::resolve_layout`), the stage before `link`.

## The gate that reads it

`crates/sigil-frontend-as/tests/failed_run_reports_everything.rs`,
`the_witness_451cb3e2_cites_is_refused_by_the_front_end`, assembles
`control.asm` from this directory and holds the front end to exactly those two
diagnostics. Reverting the gate to `!force_relocate && poison.is_empty()` turns
it red with the front end answering `Ok`.
