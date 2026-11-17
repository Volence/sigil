# Step 0 soundness parcel (branch `parcel/step0-soundness`)

Plan: `.superpowers/sdd/2026-11-16-sigil-soundness/`. Goal: a green `sigil build` is trustworthy.
Not byte-changing: every task ends with `scripts/step0_shapes_cmp.sh` printing `SHAPES 7/7 IDENTICAL`.

## Task 0: baseline and byte-neutral harness

- Branch cut from sigil master `cc1cc6b3` (= `94cf811d` + the plan doc only).
- BASE binary: detached worktree of `94cf811d`, `CARGO_TARGET_DIR=<this worktree>/.target-base`,
  `sigil --version` closure-revision `07f9caaa2f2c6ffa3ede32d5cd16e13a8f597037` (same as shipping).
- Aeon reference tree: `/home/volence/sonic_hacks/.aeon-sigil-ref-936439fa`, provisioned at aeon
  `936439fa` (see deviation). `provision-aeon-ref.sh` exit 0; its build reproduced s4 `193507d9/850296`.

Deviation from the dispatch: the plan said provision aeon `6db86e0d`. `provision-aeon-ref.sh` refuses any
revision not reachable from aeon `origin/master`; `6db86e0d` and `90a45adb` are local-only (origin is
`936439fa`, 2 commits behind master). `git diff --stat 90a45adb 6db86e0d` is one doc-only file, and
`origin/master..6db86e0d` touches only `docs/` (12 files, all docs), so the build inputs are identical;
the CRC rows below match the plan's expected ones, which confirms it.

### Baseline: seven shapes, base against base (wall-clock 28.9 s)

```
SHAPE plain 193507d9/850296 193507d9/850296 SAME
SHAPE debug ce326232/870560 ce326232/870560 SAME
SHAPE demo 2f9e2d4c/99994 2f9e2d4c/99994 SAME
SHAPE demo_debug 64717bac/106981 64717bac/106981 SAME
SHAPE config_a 6be62db3/870972 6be62db3/870972 SAME
SHAPE config_b d888a312/617813 d888a312/617813 SAME
SHAPE lean 60a3552b/798202 60a3552b/798202 SAME
SHAPES 7/7 IDENTICAL
STEP0_CMP_END rc=0
```

### RF proof for the script (invariant 8)

Mutation: `--cand` wrapper (run-unique, untracked: `.scratch-rf/mut_cand.sh`) that runs the real binary,
then flips byte `0x200` of its `-o` file (`b[0x200]^=0xFF`). Result: every shape DIFFER,
`SHAPES 0/7 IDENTICAL`, `STEP0_CMP_END rc=1`. Base against base: `SHAPES 7/7 IDENTICAL`, rc 0.
A missing reference tree prints `COULD NOT RUN setup: ...`, rc 2.

### Plan baseline table (measured 2026-11-16 at `94cf811d`)

| Item | Result |
|---|---|
| Citations | all hold at `94cf811d`: `lower/code.rs:1396-1405`, `m68k.rs:13-21,1907-1913`, `frontend-as/src/eval.rs:3333-3338`, `map.rs:99-101,135-136`, `native.rs:4479`, `main.rs:2153-2375,2236-2248,2525-2575`, `seam1.rs:1030-1060`, `test_g4_final_objects_port.rs:138,417-433`; `winptr` mask is `eval/builtins.rs:457-476` |
| A1 probes | `move.w #$12345` `303C 2345`; `move.b #300` `123C 002C`; `andi.b #$1FF` `0200 00FF`; `cmpi.b #300` `0C00 002C`; `move.b #-129` `143C 007F`; `move.w #$12345, sr` `46FC 2345`; `andi.b #$1FF, ccr` `023C 00FF`; `addi.w #-32769` `0640 7FFF`; `btst #40, d0` `0800 0028`; all exit 0. AS route refuses the first five |
| A1 corpus | sized `.b`/`.w` immediates: 0 of 879 resolvable out of range (566 unresolvable by grep); 10 `.b #~X` sites in range; bit numbers 0 of 209 (18 not resolvable) |
| A2 probes | `z80probe` z2 exit 1; z3 exit 0 byte-identical to z1. Bank 1: `$B8000-$BDC5C`, 9,124 B headroom before `SongBank2_Head` `$C0000`; config_b 617,813 B ends below `$B8000`; a `when` key on `[[region]]` is silently accepted |
| A3 corpus | `[bus.*]`, `[branch.condition-constant]`, `[call.slot-type-mismatch]/[option.unguarded-use]`: 0 firings in 7 shapes; `[proc.clobber-undeclared]` closure 0 |
| A4 probes | p8b `sizeof(S)` = 12 (want 6); p2d `9, 9` (want 5, 5); p2e `6` (want 4); corpus: 0 differing free names (ESTIMATED) |
| A5 corpus | 27 procs excluded (26 `Seq_Op_*` + `Seq_ContinueFetch`, `engine/sound/sound_sequencer.emp`); `@allow("clobbers.computed-dispatch", ...)` builds exit 0 today |
| A8 cause | CI red since `b4472d9c` (2026-09-28); one test, `g4_objects_regions_match_reference`, panics at `test_g4_final_objects_port.rs:138` with no reference guard |
| A9 drift | goldens pin aeon `1ee78b88`; aeon 64 commits ahead. s4, demo, config_b equal goldens; lean `60a3552b` moved |

## Task 1 (A3): the three printed check families gate the build

Commit `339054c6`. `run_contract_gate` (`crates/sigil-cli/src/main.rs`) refuses `[bus.*]`,
`[branch.condition-constant]` and `[call.slot-type-mismatch]`/`[option.unguarded-use]` as zero-firing
families through `refuse_located`, one `file:line:col: <describe>` line per firing. Measured before gating:
0 firings of each in all 7 shapes (table above), so no aeon change. Closes ledger row
`BUS-NET-BUILD-GATE-ROW` (`campaign-gap-ledger.md`). No `docs/QUEUE.md` row carried this item; the
ledger row is the one the plan meant.

Test `crates/sigil-cli/tests/printed_families_gate.rs`: RED before the gate (first test panicked at the
bus-header assertion, `printed_families_gate.rs:132`; the two controls passed), GREEN 3/3 after.

### RF proof (invariant 8)

```
$ git status --short
$ sed -i 's/if refuse_located("\[bus\.\*\]/if false \&\& refuse_located("[bus.*]/' crates/sigil-cli/src/main.rs
$ git diff --stat
 crates/sigil-cli/src/main.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
$ grep -n 'if false && refuse_located' crates/sigil-cli/src/main.rs
2255:    if false && refuse_located("[bus.*] Z80-bus machine-state firings",
$ cargo test -p sigil-cli --test printed_families_gate
test the_build_gate_refuses_each_printed_family_with_its_location ... FAILED
panicked at crates/sigil-cli/tests/printed_families_gate.rs:132:5:
the gate must refuse the bus family with its count, got:
error: [branch.condition-constant] firings, 1 firing(s); ...
error: [call.slot-type-mismatch]/[option.unguarded-use] firings, 1 firing(s); ...
test result: FAILED. 2 passed; 1 failed; 0 ignored
$ git restore --source=HEAD -- crates/sigil-cli/src/main.rs
$ git status --short
$ cargo test -p sigil-cli --test printed_families_gate
test result: ok. 3 passed; 0 failed; 0 ignored
```

### BN (candidate `sigil 0.1.0 (339054c6)`, wall-clock 35.3 s)

```
SHAPE plain 193507d9/850296 193507d9/850296 SAME
SHAPE debug ce326232/870560 ce326232/870560 SAME
SHAPE demo 2f9e2d4c/99994 2f9e2d4c/99994 SAME
SHAPE demo_debug 64717bac/106981 64717bac/106981 SAME
SHAPE config_a 6be62db3/870972 6be62db3/870972 SAME
SHAPE config_b d888a312/617813 d888a312/617813 SAME
SHAPE lean 60a3552b/798202 60a3552b/798202 SAME
SHAPES 7/7 IDENTICAL
STEP0_CMP_END rc=0
```
The cmp script leaves `SIGIL_CONTRACTS` unset, so the new gate ran in all 14 builds and refused none.

### AG (throwaway aeon worktree at `936439fa`, `SIGIL_BUILD`/`SIGIL_EMIT` = `.target-land` `339054c6`)

| Run | exit | wall-clock |
|---|---|---|
| `tools/landing_build.sh` | 0, `finished=0` (pytest 35 ran / 0 failed / 1 exempted) | ~7.8 min |
| `./build.sh` | 0 | 380 s |
| `DEBUG=1 ./build.sh` | 0 | 401 s |
| `./build.sh demo` | 0 | 405 s |
| `DEBUG=1 ./build.sh demo` | 0 | 370 s |

No run printed `contract closure gate SKIPPED` and none printed a `zero-firing by contract` refusal:
the three families fire 0 times in every aeon shape, now as an enforced fact rather than a printed one.

### SS (`scripts/landing-run.sh --aeon .aeon-sigil-ref-936439fa`), candidate against base

| Tree | suites (launched/reported) | passed | failed | ignored | RESULT | wall-clock |
|---|---|---|---|---|---|---|
| `339054c6` (this task) | 522 / 522 | 5,803 | 67 | 3 | FAILED, cargo 101, clippy 0, ledger 0 | 511 s |
| `94cf811d` (base) | 521 / 521 | 5,800 | 67 | 3 | FAILED, cargo 101, clippy 0, ledger 0 | 511 s |

The 67 failing test names are IDENTICAL in both runs (sorted `diff` empty): golden/reference drift at aeon
`936439fa` (region/anchor/full-file goldens pinned at `1ee78b88`, the A9 item, plus provenance-tip and
pins currency), not this task. The delta is exactly the new suite, `printed_families_gate`, 3/3 passing.
No SS output contains a `zero-firing by contract` refusal. SS is therefore not GREEN at aeon HEAD before
or after this task; Task 6 (A9) owns it.

## Task 2 (A7): the `[proc.clobber-undeclared]` refusal says where

Commits `76a2f5cd` (feature) and `f97406ea` (test fixture fix, below). The closure gate used to refuse
with `N firing(s)` and nothing else. It now refuses through `refuse_located`, one line per firing:

```
error: [proc.clobber-undeclared] closure firings, 3 firing(s); this family is zero-firing by contract:
  <file>:7:5: [proc.clobber-undeclared] `P` destroys `d1` through `Q`, which its clobbers(...) does not declare
  <file>:11:5: [proc.clobber-undeclared] `R` destroys `d2` in its own body, which its clobbers(...) does not declare
  <file>:15:5: [proc.clobber-undeclared] `U` reaches an indirect call with no declared bound, so it may destroy every register; bound the call with `as Type` or declare `@allow("clobbers.unanalyzable", "<reason>")`
```

- `ProcNode.decl_span` (68k proc, extern, and the Z80 seam-1 node: `seam1.rs` is a sixth path the plan
  did not list, needed because that literal names every field); `Firing` gains `span` and `via`.
- `closure::first_carrier`: first callee in sorted `direct_callees` whose effective set holds the
  register, else `falls_into`, else `None`; it resolves names with the fixpoint's own rule
  (`resolve_key_in`, which `resolve_callee_key` now delegates to). A transitive register no callee
  carries can only have come through a BOUNDED indirect site; it is named `jsr (aN) as Type` rather than
  falsely "in its own body" (plan text had no form for this case; unit-tested).
- `describe_clobber(f, code)` in `main.rs` for Task 7; `--report contracts` prints `... via <carrier>`.
- `empty_gate` rows now return `bool` (`failed |= ...`) so the located row keeps its place, on one line.
- `refuse_located` = `located_refusal` (pure render) + print; its `(no span recorded)` branch is
  unit-tested (Task 1 review carry-over), both for `span: None` and a span whose source does not read.
- Finding: a proc declaration span starts at `proc`, after `pub` (column 5 in the fixture).

Corpus: 0 closure firings in all 7 shapes (plan baseline, MEASURED), so nothing new fires.

Test `crates/sigil-cli/tests/clobber_gate_location.rs`: RED before (panicked at the header assertion,
`clobber_gate_location.rs:100`, stderr showing the old `3 firing(s); ... by contract.` line; both
controls passed), GREEN 3/3 after. `sigil-frontend-emp --no-fail-fast` (with `AEON_DIR` = the ref
tree): 138 binaries, 2,787 passed, 0 failed.

### RF proof (invariant 8)

```
$ git status --short
$ sed -i 's|refuse_located("\[proc.clobber-undeclared\] closure firings".*|empty_gate("[proc.clobber-undeclared] closure firings", report.firings.len());|' crates/sigil-cli/src/main.rs
$ git diff --stat
 crates/sigil-cli/src/main.rs | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)
$ grep -n 'empty_gate("\[proc.clobber-undeclared\]' crates/sigil-cli/src/main.rs
2249:    failed |= empty_gate("[proc.clobber-undeclared] closure firings", report.firings.len());
$ cargo test -p sigil-cli --test clobber_gate_location
test the_clobber_refusal_names_proc_register_path_and_line ... FAILED
panicked at crates/sigil-cli/tests/clobber_gate_location.rs:99:5:
error: [proc.clobber-undeclared] closure firings, 3 firing(s); this family is zero-firing by contract.
test result: FAILED. 2 passed; 1 failed; 0 ignored
$ git restore --source=HEAD -- crates/sigil-cli/src/main.rs
$ git status --short
$ cargo test -p sigil-cli --test clobber_gate_location
test result: ok. 3 passed; 0 failed; 0 ignored
```
The plan expected the FAIL on the `P` line; it fires one assertion earlier, on the located header
(`...by contract:` vs the old `...by contract.`), which precedes it. Same mutation, same cause.

### BN (candidate `sigil 0.1.0 (f97406ea)`, wall-clock ~30 s; also 7/7 at `76a2f5cd`)

```
SHAPE plain 193507d9/850296 193507d9/850296 SAME
SHAPE debug ce326232/870560 ce326232/870560 SAME
SHAPE demo 2f9e2d4c/99994 2f9e2d4c/99994 SAME
SHAPE demo_debug 64717bac/106981 64717bac/106981 SAME
SHAPE config_a 6be62db3/870972 6be62db3/870972 SAME
SHAPE config_b d888a312/617813 d888a312/617813 SAME
SHAPE lean 60a3552b/798202 60a3552b/798202 SAME
SHAPES 7/7 IDENTICAL
STEP0_CMP_END rc=0
```

### SS (`scripts/landing-run.sh --aeon .aeon-sigil-ref-936439fa`)

| Tree | suites (launched/reported) | passed | failed | ignored | RESULT | wall-clock |
|---|---|---|---|---|---|---|
| `76a2f5cd` | 523 / 523 | 5,809 | 68 | 3 | FAILED, cargo 101, clippy 0, ledger 0 | 476 s |
| `f97406ea` (tip) | 523 / 523 | 5,810 | 67 | 3 | FAILED, cargo 101, clippy 0, ledger 0 | 431 s |

At `76a2f5cd` one NEW name against Task 1's 67: `no_source_in_this_crate_writes_a_file_by_truncation`
(`atomic_output_gate.rs` scans `#[cfg(test)]` modules too; the new `refuse_located` unit test wrote its
fixture with `std::fs::write`). `f97406ea` routes it through `install_artifact`. At the tip the 67
failing names (taken from cargo's `failures:` lists, as a multiset; the script's own deduplicated
`FAILING TESTS (63)` list drops `mt_bank_debug_matches_reference`, whose `test ... FAILED` line was
interleaved) are IDENTICAL to `.scratch-t1/failed_base.txt`. Delta vs Task 1: +1 suite, +7 passed (the
3 integration, 3 `closure::tests`, 1 `located_refusal` unit test). No SS output contains a
`zero-firing by contract` refusal.

### AG (throwaway aeon worktree at `936439fa`, `SIGIL_BUILD`/`SIGIL_EMIT` = `.target-land` `f97406ea`)

| Run | exit | wall-clock |
|---|---|---|
| `tools/landing_build.sh` | 0, `finished=0` (pytest 35 ran / 0 failed / 1 exempted) | 473 s |
| `./build.sh` | 0 | 381 s |
| `DEBUG=1 ./build.sh` | 0 | 394 s |
| `./build.sh demo` | 0 | 362 s |
| `DEBUG=1 ./build.sh demo` | 0 | 386 s |

Every log names `sigil f97406ead25f` and no other assembler revision; both binaries' md5 at the end
equal their md5 at the start; no run printed `contract closure gate SKIPPED` or a `zero-firing by
contract` refusal. A FIRST AG attempt was discarded: the SS run shared `.target-land` and rebuilt the
release binary at `f97406ea` mid-run, so its `./build.sh` began on `76a2f5cd` and its gates read
`f97406ea`. It was killed and the whole AG rerun with no cargo in flight (lesson for later tasks: do not
overlap SS with AG on the same target dir).

### Fix round 1 (`5284af4c`)

Review finding: a self-recursive proc named itself as the carrier (`S destroys d3 through S`). Under the
controller ruling, `first_carrier(name, ..)` now returns `None` when the proc writes `reg` itself; otherwise
it takes the first sorted callee holding `reg`, skipping self-edges, then `falls_into`, then the bounded-indirect
fallback. New tests cover self-recursion and mutual recursion, the fixture's `S`, and the `--report contracts`
`via` row. RF: removing both guards turns 3 tests red; after the restore they are green. frontend-emp 2789/0;
sigil-cli 850 passed / 57 failed (all baseline names); BN 7/7 IDENTICAL at `5284af4c`.

### Booked, not in scope

- Dropping the 63 per-file `[proc.clobber-undeclared]` warnings the closure finds clean (Sigil audit
  plan item 5, second half). Unowned in step 0; candidate for a later QUEUE row.
