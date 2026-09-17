# 2026-09-17, `SWEEP-NIGHTLY`: the switch-matrix sweep gets a clock and reaches build.lua

Branch `parcel/sweep-nightly`, based on master `d7e6aa15`. Scratch for every run below:
`~/sonic_hacks/.scratch/sweep-nightly/` (not committed).

## What landed

| piece | what it does |
|---|---|
| `scripts/switch_matrix_sweep.py` | derives build.lua's Settings block as a second option source (L1 to L3); re-derives p2bin arguments per leg; reads each corpus from a named commit extracted afresh; per-leg `LEG_START` / `LEG_REPORTED` / `LEG_ERROR` markers; `--derive-only`; `ACK_DISAGREE` entries may pin diagnostic text; self-test controls C10a-c, C11a-c, C12 |
| `scripts/nightly_switch_sweep.sh` | builds sigil from a detached `origin/master` into its own target, pins both corpora to committed SHAs, runs the sweep with `--cross`, reconciles the run's own markers, exits 0 / 1 / 2 and notifies on 1 and 2 |
| `scripts/systemd/sigil-switch-sweep.{service,timer}` | 02:17 daily. Committed, **not installed**; `scripts/systemd/README.md` has the install lines |

## The build-script settings found

Derived from each corpus's build.lua, at s1disasm `f6ece657`, s2disasm `e45ebf33`:

| corpus | `.lua` files cross-checked | setting | literal | derived | legs |
|---|---|---|---|---|---|
| s1disasm | 5 | `improved_dac_driver_compression` | `false` | domain {false, true} | 1 new phase-1 leg; corners 288 to 576 |
| s2disasm | 5 | `improved_sound_driver_compression` | `false` | domain {false, true} | 1 new phase-1 leg; corners 96 to 192 |
| s2disasm | | `music_buffer_address` | `0x1380` | UNREADABLE, acknowledged | none |
| s2disasm | | `music_buffer_size` | `0x7C0` | UNREADABLE, acknowledged | none |

The two number literals are consistency constants: build.lua says each "Should always" match a
fact of the driver source, so there is no second legal value to sweep. `.asm` options unchanged:
s1disasm 11 (7 swept, 9 arms), s2disasm 10 (6 swept, 7 arms).

## What the widened reach found

Both new arms are `SIGIL-DECLINED`, and this was red before it was acknowledged (first green-attempt
run: "UNACKNOWLEDGED s1disasm s1disasm-build.lua:improved_dac_driver_compression-1: SIGIL-DECLINED",
and the same for s2disasm). At `true` the stock build hands `kosinski-optimised` /
`saxman-optimised` to p2bin and its image moves (Sonic 1 `afe05eee` to `faa36f4d`, 524,288 bytes;
all 192 Sonic 2 corners are distinct stock images). sigil refuses before assembling:

```
error: `-z=0,kosinski-optimised,Size_of_DAC_driver_guess,after`: `kosinski-optimised` is a p2bin
format sigil does not implement; sigil implements uncompressed, kosinski, saxman-bugged
```

A loud refusal, not a wrong ROM, so both legs are acknowledged with that text pinned. They have
compared no byte; the retirement condition is the `SWEEP-NIGHTLY` row in the campaign gap ledger.

In `--cross`, the composition prediction reads that refusal off phase 1 and holds on every corner
(Sonic 1: 528 held, 0 broke, 48 not asked because the stock toolchain declined; Sonic 2: 192 held,
0 broke). Corners both toolchains built: Sonic 1 264, all agreeing; Sonic 2 48, all agreeing. The
Sonic 1 stock-decline rule now covers 48 corners (24 per compressor value), and sigil also declined
all 48.

Found and fixed on the way, each in the first commit's body: a reused pristine corpus tree could
be printed under a commit it did not hold; a fresh `--scratch` crashed on the first leg (no `logs/`);
an uncaught exception exited 1, the status for a finding; s2 build.lua:53's table field
`improved_sound_driver_compression = improved_sound_driver_compression,` was first refused as a
second assignment.

## Wall time and the full hand runs

Both runs through the real job, `SIGIL_SWITCH_SWEEP_HOME` / `_STATE` pointed into scratch, exit 0:

| sigil ref | start | end | wall | legs |
|---|---|---|---|---|
| `ca1d5eb2` | 2026-09-17T04:09:49-04:00 | 04:31:55 | 22m6s (cold lane target) | 808 (790 planned + 18 rescue, 0 errored) |
| `7728ba2e` | 2026-09-17T04:38:41-04:00 | 05:00:47 | 22m6s | 808 (790 planned + 18 rescue, 0 errored) |

`aeon-effects-gates` fired at 04:17 inside the first run and the load average stood near 6 to 7
throughout both, so 22 minutes is a contended figure. The sigil build inside it took 15.65 s into
the freshly created lane target and 1.54 s incrementally (cargo's own `Finished` lines). The
earlier estimate of "about a quarter of an hour" for `--cross` was for 384 corners; the widened space
is 768. Phase 1 alone (no `--cross`, not the nightly shape) measured 84 s for 40 legs.

Verdict line of the second run, as written to the job's log:

```
OK at sigil 7728ba2e (7728ba2e..., fetched) / s1disasm f6ece657 (working tree: 4 uncommitted
entries, not measured) / s2disasm e45ebf33 (working tree: 0 uncommitted entries, not measured):
808 legs (790 planned + 18 rescue, 0 errored); s1disasm: 11 options, 1 build-script settings,
576 corners;s2disasm: 10 options, 3 build-script settings, 192 corners; started ..., 22m6s wall
```

## Red-first proofs

### The derivation, mutating a scratch clone of s1disasm (never the real corpus)

Each mutation is a commit in `.scratch/sweep-nightly/proof/s1disasm`, based on `f6ece657`, run with
`--corpus s1disasm=<clone>@<sha> --corpus s2disasm=<real>@HEAD --derive-only`. Restored by running
the same command at the clone's committed baseline `f6ece657`: exit 0, `found=3 acknowledged=3 MATCH`.

| commit | line from disk | result |
|---|---|---|
| `7292d00d` | `13:local proof_new_toggle = false` | master's sweep (`d7e6aa15`) plans "9 arms -> 9 flip legs" and never names it. The new sweep derives `build.lua:proof_new_toggle = 0 domain [0, 1] arms [1]`, `corners=1152`, and `--only` runs the leg: edit `'local proof_new_toggle = false'` to `'... true'`, both builds, `VACUOUS` (nothing reads it), table row `NOT-MEASURED` |
| `93ce9328` | `13:local proof_compressor_name = "kosinski"` | exit 1: "unreadable-domain acknowledgements are stale: found-not-acknowledged=[('s1disasm', 'build.lua:proof_compressor_name')]" |
| `89ce62d7` | `22:local proof_stray_toggle = true` (after the block) | exit 2: "a column-0 boolean local outside build.lua's Settings block is a toggle this sweep cannot reach: ['build.lua:22 local proof_stray_toggle = true']" |
| `6962f030` | `11:local proof_derived = improved_dac_driver_compression and 1 or 2` | exit 2: "build.lua:11 is inside the Settings block but is not a column-0 `local <name> = <literal>` ..." |

The working tree is not read: with the clone at `f6ece657` and `local proof_stray_toggle = true`
appended UNCOMMITTED (`git status`: ` M build.lua`), the run at `@HEAD` exits 0 and the extracted
tree holds 0 occurrences. The positive control is `89ce62d7`, the same line committed, which aborts.

### The job's own exit-2 paths

Before any build: `lua` hidden from PATH gives "COULD NOT RUN: no `lua` on PATH, and the stock
reference build of every leg is each corpus's build.lua"; `S1DISASM_DIR` at a missing directory gives
"COULD NOT RUN: the s1disasm corpus checkout could not be resolved (set S1DISASM_DIR)" with the
resolver's "suite-paths: REFUSING, S1DISASM_DIR=... is not the s1disasm checkout (no such
directory)" in the log; a nonexistent `SIGIL_SWITCH_SWEEP_S2_REF` gives "COULD NOT RUN: s2disasm ref
'000...1' does not name a commit in .../s2disasm".

After the sweep, the subject mutated is the sweep script, in throwaway commits made with
`git commit-tree` off `ca1d5eb2` (objects only, no ref, no worktree) and run through the real job by
SHA. All exit 2:

| commit | mutation (from `git show <sha>:scripts/switch_matrix_sweep.py`) | diagnostic |
|---|---|---|
| `e5004f4b` | `if False and a.cross and not a.only:` (the sweep still prints SWEEP PASSED) | "2 corpora named but 0 cross products ran" |
| `10735105` | `for corner in corners[:3]:` | "the plan lines require at least 790 legs, 46 ran", size "46 legs (790 planned, SHORT by 744, 0 errored)" |
| `a420221b` | 3 corners with a matching plan line, and `LEG_REPORTED` skipped for `-shipped` legs | "46 legs started but 44 reported and 0 errored" |
| `bdbb1d0c` | `corpora = []` | "2 corpora named but 0 measured (0 population lines); ... zero legs launched" |
| `ca1d5eb2` (unmutated) | python and its `timeout` SIGKILLed after the first `LEG_START` | "the sweep timed out or was killed (exit 137)" |

`--selftest-fail` exits 1 and writes "SELFTEST: the failure-notification path works".

## Suites

The lints that read `scripts/` and the lane files, run on the branch with `--no-fail-fast`:
`drift_nightly_harness`, `pipefail_sigpipe_lint`, `scripts_name_their_tree`,
`shared_target_defaults`, `skip_marker_lint`, `source_gate_classification`: 6 binaries, 39 passed,
0 failed, 0 ignored. No Rust source changed. The sweep has no cargo regression test (it needs `lua`
and the corpora); its own self-test ran green at the head of every run above.
