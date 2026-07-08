# Plan 7 #7-pre — the L-H.1 final-size placement fix: implementation notes

Worktree `/home/volence/sonic_hacks/sigil/.worktrees/plan7-item7-banks`, branch
`plan7-item7-banks`. Plan:
docs/superpowers/plans/2026-07-08-spec2-plan7-item7pre-placement-fix.md.
RED evidence recorded per task, per the 2026-07-08 here-fix / item9b precedent.

## T0 — baseline probes

Verified on master `dfe6e7b` (the main checkout at `/home/volence/sonic_hacks/sigil`,
clean, on `master`, matching this worktree's fork point):

- `cargo test --workspace --no-fail-fast` → EXACTLY 4 failing tests, all
  allowlisted upstream aeon strlen drift, zero others:
  - `full_build_reproduces_sound_driver_regions`
  - `vector_table_matches_reference_rom_first_256_bytes`
  - `full_debug_rom_matches_assembled_reference`
  - `full_rom_matches_assembled_reference`
- `cargo clippy --workspace --all-targets -- -D warnings` → clean (no
  warnings, no errors).

Both re-confirmed independently in the worktree (T0, 2026-07-08, HEAD =
`78b0655`, content-identical to master `dfe6e7b` apart from the two plan
docs): same 4 named reds, nothing else; clippy clean.

### `scripts/corpus_bytediff.sh` — the probe

New script, plain bash (`set -u`, no cleverness). Builds `sigil-cli` in BOTH
this worktree and the pristine master checkout
(`/home/volence/sonic_hacks/sigil`), then runs each tree's own
`target/debug/sigil` binary against the SAME source files (the worktree's
copies) for:

- every `examples/*.emp` single-file build (`sigil emp <f> -o <tmp>`), and
- the two standing game invocations (`--root examples/game --prelude
  prelude`) for `examples/game/badniks/pitcher_plant.emp` and
  `examples/game/badniks/pitcher_plant_script.emp`.

Each pair is byte-diffed with `cmp`. Verdict per file: `IDENTICAL` /
`DIFFERS` / `SKIPPED` (master's binary failed to compile that file — does
not affect exit status). Exits nonzero iff any file `DIFFERS`.

### T0 run (worktree == master content-wise; sanity that the probe itself works)

```
== single-file examples (examples/*.emp) ==
SKIPPED  composition_pitcher_plant.emp (master's binary failed to compile it)
IDENTICAL dispatch.emp
IDENTICAL guards.emp
SKIPPED  main.emp (master's binary failed to compile it)
IDENTICAL offset_table.emp
IDENTICAL prelude.emp
IDENTICAL reach_branches.emp
IDENTICAL sst_overlay.emp
== game invocations (--root examples/game --prelude prelude) ==
IDENTICAL pitcher_plant.emp
IDENTICAL pitcher_plant_script.emp
RESULT: all identical (SKIPPED files, if any, excluded)
EXIT=0
```

`composition_pitcher_plant.emp` and `main.emp` are pre-existing corpus
failures on master itself (unrelated to this branch — `unknown name
\`timer\`` / undeclared-fallthrough diagnostics, and a missing type
annotation on `ObjectIndex`, respectively), so both binaries fail on them
identically and the probe correctly reports `SKIPPED` rather than
`DIFFERS`. All 8 buildable targets (6 single-file + 2 game) are
byte-identical, confirming the probe works before this branch touches any
placement code.

## RED evidence (filled in per task)

| Task | Test | RED command | RED result | GREEN commit |
|------|------|-------------|------------|---------------|
|      |      |             |            |               |
