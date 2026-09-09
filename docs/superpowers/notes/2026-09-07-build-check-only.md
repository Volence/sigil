# `sigil build --check`: decide every guard, emit nothing (2026-09-07)

Parcel `parcel/build-check-only`, base sigil master `d54bd396`. Closes ledger row
`CHECK-ONLY-INVOCATION` (`campaign-gap-ledger.md`) and question (k) of
`2026-09-07-link-assert-reporting.md`. Aeon's ask: a per-game check-only run that
decides every `ensure` and every `LinkAssert` without writing a ROM, so
`engine/system/z80_init.emp`'s `ensure(extern("Z80_IDLE_SIZE") == 40, ..)` (never
in the shipped sonic4 closure, `when = "sound_off"`) can be decided under
`--game demo` on a per-commit path.

## The flag

`sigil build --aeon <dir> [--game sonic4|demo] [--debug] [--config-a|--config-b|--lean]
[--extra-entry <module|path.emp>]... --check`

It rides every target selector and `--extra-entry`. It refuses `-o` and
`--emit-lst` (it writes neither) and `--report` (a different run), each at parse
time with a usage error (exit 2), the same shape `--report` already used. There is
no `--help` in this CLI; the usage block prints on any parse error, and `sigil
build --help` reaches it through the unknown-argument path (exit 2). Its text now:

> **Superseded 2026-09-09 (`parcel/cli-help`).** The two sentences above are no
> longer true of the binary. `sigil --help`, `-h` and `help` print help at exit 0,
> and `sigil build --help` prints the build page directly rather than reaching a
> usage block through the unknown-argument path. Every usage text now lives in the
> `ENTRIES` table in `crates/sigil-cli/src/main.rs`, which is also what `main`
> dispatches on; the block quoted below is the shape it had at this note's date and
> is kept as the record of what `--check` added to it.

```
usage: sigil build --aeon <dir> [-o <out.bin>] [--emit-lst <lst>] [--game sonic4|demo] [--debug] [--config-a|--config-b|--lean] [--report ram|contracts] [--extra-entry <module|path.emp>]... [--check]
note:  --extra-entry evaluates the NAMED module's comptime guards; the named module must emit nothing (its own imports are not checked)
note:  --check decides every ensure and LinkAssert against final post-relaxation placement and writes no ROM; a green check proves nothing about region budget or overlap, image bounds, the checksum or the closure gate, and is not a statement that the game builds
env:   SIGIL_WARNINGS=off|summary|full  (warn-tier detail; default summary)
```

## What it runs, and where it stops

`native::check_chained(aeon, profile)` = the chained build's own prefix, extracted
as `resolve_chained`: `emit_generated` (sound-on shapes only), `assemble_as_side`,
`build_emp`, the declared chain (`true_bases_by_index`, `declared_spans_by_index`,
`apply_declared_chain`), `resolve_layout`, `check_link_asserts`, the warn tier,
`declared_chain_drift_verdict`, `enforce_inapplicable_allowlist_against`. Then it
stops. `build_rom_chained_with_listing` now calls the same `resolve_chained` and
continues into the listing, `link`, `validate_placement`,
`validate_resolved_alignment`, `validate_sound_fold`, `check_object_bank_budget`
and `emit_rom`, in the order it always had; the full build's calls, order and
error strings are unchanged. The CLI's check path does not run the contract
closure gate (`run_contract_gate`), does not build a listing, does not append
deb2, does not fold a checksum, and has no output path to write.

One correction to the brief: `resolve_frozen_sections` runs the same five steps
but DROPS the link asserts and the warn tier (it is a placement helper), so a
check built on it literally could decide nothing. The check shares the build's
own prefix instead, which is the same five steps with the asserts kept.

## What it says about itself

stderr, first line of every check run (one line, wrapped here):

```
check: demo plain: deciding every ensure and LinkAssert against final post-relaxation placement, then stopping before the link: a green check proves nothing about region budget or overlap, image bounds, the checksum or the contract closure gate, and is not a statement that the game builds
```

For a sound-on shape a second stderr line follows, because `emit_generated` is a
precondition of the `.emp` build and this parcel does not change it:

```
check: sonic4 plain: emit_generated writes the sound artifacts into /home/volence/sonic_hacks/.aeon-check-probe (a precondition of the .emp build), the one write this run makes
```

stdout, last line of a green run (the census):

```
checked: demo plain: 2125 ensure verdict(s) at comptime, 274 LinkAssert(s) decided at link, 0 LinkAssert(s) inapplicable (extern not defined in this link, allowlisted)
```

A failed run prints the guard and no census line. Exit 0 when every decided guard
holds; exit 1 with the same rendered lines the full build prints (the
`declared-chain drift guard FIRED: N error(s):` header and one
`path:line:col: [Error] msg` per guard, or `build_program: N error(s);` for a
comptime guard), under `error: native check (<label>): `.

## Where the counts come from

Never a grep or an AST count. `Evaluator.guards_decided` increments on every
`ensure`/`ensure_fatal` evaluation that reaches a `Bool` verdict (passed or
failed; a Poison condition or an arity error is not a verdict). It is drained
beside `take_link_asserts` at exactly the two lowering sites that drain the
asserts (item-position guards in `lower_item_guard`, data initializers in
`lower_data_item`), onto `sigil_ir::Module::comptime_guards`, and summed over the
reachable modules by `resolve::build_program_open_embed_counted` (the existing
three-tuple entry points are unchanged wrappers over it). So a guard evaluated at
those sites is counted at comptime or deferred to link, never both and never
neither. The count is of EVALUATIONS: a guard inside a comptime `fn` counts once
per call, which is why pulling `games.sonic4.constants` (32 item-level
`ensure`s) into the demo closure raised the count by 58, not 32.

`LinkAssert(s) decided` = the list `check_link_asserts` folded minus the
inapplicable subset the drift verdict handed back (extern not defined in this
link, on the profile's allowlist or the run would already have failed). Both
columns come from `GuardCensus::from_verdict`, the one place the arithmetic lives.

Scope, stated: guards reached through other evaluator entry points (dispatch and
offset tables, `asm {}` splices, const-fold probes) are decided as they always
were but are not counted; they are also not where the deferred asserts are
collected. The count line names what it counts.

## Red-first (the negation aeon uses), demo shape, aeon `6fb8fdc3`

Probe tree: a detached worktree of aeon `origin/master` at `6fb8fdc3`, created and
removed by this parcel; the mutation was applied to the worktree's copy and
restored from the committed baseline with `git checkout`, never committed.

```
--- a/engine/system/z80_init.emp
+++ b/engine/system/z80_init.emp
@@ -62,7 +62,7 @@
-ensure(extern("Z80_IDLE_SIZE") == 40, "boot_data.emp Z80_IDLE_SIZE mirror drifted from z80_init.emp's 40-byte idle body")
+ensure(!(extern("Z80_IDLE_SIZE") == 40), "boot_data.emp Z80_IDLE_SIZE mirror drifted from z80_init.emp's 40-byte idle body")
```

`sigil build --aeon <probe> --game demo --check` under the mutation, exit 1:

```
error: native check (demo plain): declared-chain drift guard FIRED: 1 error(s):
  /home/volence/sonic_hacks/.aeon-check-probe/engine/system/z80_init.emp:65:1: [Error] boot_data.emp Z80_IDLE_SIZE mirror drifted from z80_init.emp's 40-byte idle body
```

After `git checkout -- engine/system/z80_init.emp` (status clean), exit 0:

```
checked: demo plain: 2125 ensure verdict(s) at comptime, 274 LinkAssert(s) decided at link, 0 LinkAssert(s) inapplicable (extern not defined in this link, allowlisted)
```

The not-decided control, same tree, `SIGIL_WARNINGS=full`: `games.sonic4.constants`
is outside the demo closure. Without a flag the run reports

```
.../games/sonic4/config/constants.emp:22:1: warning: [module.unreachable] module `games.sonic4.constants` is outside this profile's `use` closure, so its 32 `ensure` guard(s) are never evaluated for this target, they cannot fail here, whatever they assert. (...)
checked: demo plain: 2125 ensure verdict(s) at comptime, 274 LinkAssert(s) decided at link, 0 LinkAssert(s) inapplicable (...)
```

and with `--extra-entry games.sonic4.constants` the row is gone and the census
reads `2183 ensure verdict(s) at comptime, 274 LinkAssert(s) decided at link`.
The guard the brief names is in the same position under sonic4: `registry()`
excludes `engine.z80_init` from every sound-on shape (`native.rs`, the demo and
config-b registries add it explicitly), so under `--game sonic4` its extern guard
is reported `[module.unreachable]` and is absent from both columns. That sonic4
run could not be executed on this tree, see the next section.

## Timing (probe tree aeon `6fb8fdc3`, parcel binary, 16 cores)

Same tree, same binary, back to back:

| run | wall | uptime before / after |
|---|---|---|
| `--game demo --check` | 0.40 s | `10:40:56 up 14:54, load 2.43, 3.69, 4.47` / `10:40:57 ... 2.43` |
| `--game demo -o demo.bin` (full) | 0.74 s | `10:40:57 up 14:54, load 2.43` / `10:40:58 ... 2.43` |
| `--game demo --debug --check` | 0.40 s | `10:44:15 up 14:57, load 2.44, 3.32, 4.18` / `10:44:15 ... 2.44` |
| `--game demo --debug -o` (full) | 0.75 s | `10:44:15 up 14:57, load 2.44` / `10:44:16 ... 2.44` |

The saving is the tail (link, listing, validation, appendix, checksum, the
closure gate): roughly 0.35 s of 0.75 s here. The "~150 s demo build" in the ask
is not `sigil build`: aeon's own `build.sh` banner measures `sigil build (the
ROM) 1.15 s` beside `emp_expect_fail 22.69 s (20 real sigil builds)` and `pytest
tools 12.40 s`, so the per-commit cost aeon is paying lives in the verification
lanes around the build, not in the resolve `--check` shares. Whether a 0.4 s
check earns a slot on a per-commit path is aeon's call; this note only corrects
the premise.

Sonic4 shapes (plain, debug) could not be timed: on aeon `6fb8fdc3` with sigil
`d54bd396`'s pins, `emit_generated` trips first (`emit_sound_blob (blob): plain
blob is 6176 bytes, expected 6163`), the chain-refreeze lane's territory, and
BOTH `--check` and the full build fail with the identical error line and exit 1,
which is at least the failure surface being shared. The demo debug shape needed
`tools/bin/salvador` and `gen_compression_vectors.py` run in the probe tree (a
bare worktree lacks `engine/debug/generated/*.bin`).

## Byte neutrality on the shapes this tree could build

First attempt was vacuous and is struck: building the base sources in the
parcel's target dir relinked the BASE CLI against the PARCEL's already-built
library rlibs (cargo keys path packages by name, not path), so the "base" binary
carried the refactor under test. Redone with every sigil crate cleaned before
each build and each binary identified behaviourally: the base CLI refuses
`--check` (`error: unexpected argument '--check'`, exit 2), the parcel CLI
accepts it (`sigil 0.1.0 (5059979c)`). Then, same probe tree:

```
demo plain: base crc=0ad17404 len=96863  parcel crc=0ad17404 len=96863  identical=True
demo debug: base crc=2565ece2 len=103185  parcel crc=2565ece2 len=103185  identical=True
```

The four ROM shapes at landing are the controller's gate, as briefed.

## Gates, and the mutations that redden them

`crates/sigil-harness/tests/check_only_census.rs` (5 cases, tree-free, runner
`cargo test -p sigil-harness --test check_only_census`):
`comptime_verdicts_are_counted_where_asserts_are_drained`,
`a_failed_comptime_guard_is_still_a_verdict`, `every_defined_extern_guard_is_decided`,
`an_undefined_extern_guard_is_reported_not_decided` (decided 1 of 3),
`a_drifted_extern_guard_fails_the_verdict_with_its_line`.

`crates/sigil-cli/tests/build_check.rs` (4 cases, reference tree, demo, runner
`SIGIL_STRICT_GATE=1 AEON_DIR=<aeon> cargo test --release -p sigil-cli --test
build_check`): `a_clean_check_decides_both_families_and_writes_nothing` (banner
first on stderr, census on stdout, cwd empty after; the full build writes
`demo.bin` at the path a check then refuses, exit 2, file absent),
`a_failing_guard_fails_the_check_with_its_rendered_line` (`--extra-entry` on
the committed `poison_band_nested.emp`: exit 1, one `[Error]`, located
`path:line:col`, no census), `an_unreachable_guard_is_reported_not_decided`,
`every_aeon_fixture_this_file_names_still_resolves`. `sigil build` needs a tree
(map, residual, registry), so a self-contained `.emp` program cannot drive it;
the census arithmetic is the tree-free half. The grammar is pinned in
`main.rs`'s `parse_build_args_check_grammar`.

Mutation proofs, each applied to the worktree, shown applied, restored with `git
checkout` from the committed baseline, gates re-run green after:

1. `eval/guards.rs`: the `Bool(true)` arm no longer increments `guards_decided`.
   Census gate red: `left: 0, right: 3` and `left: 1, right: 3` (4 of 5 cases).
2. `main.rs`: the banner `eprintln!` removed. CLI gate red at the banner assert.
3. `main.rs`: the census prints `0, 0`. CLI gate red in two cases (`comptime > 0`,
   and the count no longer rises with `--extra-entry`).
4. `main.rs`: the check writes `demo.bin` into its cwd. CLI gate red:
   `a check wrote into its cwd: left: ["demo.bin"], right: []`.

## Totals

`SIGIL_ALLOW_PARTIAL=1`, no reference tree named (reference-dependent rows
unmeasured, expected; the controller runs the full gate), `--no-fail-fast`,
tree `5059979c` on `parcel/build-check-only`, uptime `10:50:36 up 15:03, load
13.65` to `10:51:20`:

* sigil-ir: 2 binaries, 52 passed, 0 failed, 0 ignored
* sigil-frontend-emp: 129 binaries, 2643 passed, 0 failed, 0 ignored
* sigil-harness: 48 binaries, 428 passed, 0 failed, 1 ignored
* sigil-cli: 156 binaries, 702 passed, 0 failed, 1 ignored
* `cargo clippy --release -p sigil-cli -p sigil-harness -p sigil-frontend-emp
  -p sigil-ir --all-targets -- -D warnings`: clean.
* Added lines swept for em/en dashes: 0.

`build_check.rs` was additionally run strict against the probe tree
(`AEON_DIR=<probe> SIGIL_STRICT_GATE=1`): 4 passed.

## Commits

* `ac77c1ee` build --check: decide every guard against final placement, emit nothing
* `5059979c` build --check: gate the census and the CLI surface
* this note (SHA in the final report)

## Open

* The sonic4 timing pair and the sonic4 `[module.unreachable]` row for
  `engine.z80_init` under `--check`, once the chain refreeze lands and a sound-on
  shape resolves on current aeon.
* aeon's decision on where `--check` sits in `build.sh`, given the corrected
  premise (the resolve is well under a second; the lanes around it are the cost).
* `--check` skips the closure gate by design; a `--check --contracts` that runs
  it too is a one-line addition if aeon wants a single per-commit command.
