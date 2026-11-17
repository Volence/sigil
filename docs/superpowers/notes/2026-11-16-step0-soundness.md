# Step 0 soundness parcel (branch `parcel/step0-soundness`)

Plan: `docs/superpowers/plans/2026-11-16-sigil-soundness.md` (the per-task briefs, reports and reviews
under `.superpowers/sdd/2026-11-16-sigil-soundness/` are a gitignored working dir, not part of the
record). Goal: a green `sigil build` is trustworthy.
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

## Task 3 (A1): sized immediates, bit numbers and typed consts are range-checked

Commit `86272767`. The `.emp` front end narrowed every 68k `#imm` to its field silently (plan baseline:
`move.w #$12345` `303C 2345`, `btst #40, d0` `0800 0028`, all exit 0). Now:

- `lower/code.rs`: every `M68kBackend.lower_inst` call (the 12 sites) goes through
  `lower_inst_checked` = `check_imm_ranges` then encode. `imm_bounds(size)` is the AS front end's
  `m68k_imm_bounds`; the message is its `operand {v} out of range {lo}..={hi}`, and the 32-bit narrowing
  message in `m68k_operand` takes the same form. A bit op's source bit number: 0..=31 on `Dn`, 0..=7
  otherwise, `[lower.bit-number] bit number {n} out of range 0..={hi}`. The pc-relative path
  (`lower_pcrel_ea`, not `lower_inst`, so not in the plan's 12) runs the same check, for
  `btst #n, Sym(pc)`. `ImmLink` paths encode a zero placeholder, so the check never sees a link-time value.
- `eval/const_arity.rs`: a const typed as a bare primitive is range-checked,
  `[const.out-of-range] {v} does not fit {ty} ({lo}..={hi}), in '{name}'`.
- Test `crates/sigil-frontend-emp/tests/imm_range.rs`: RED 6 failed / 2 passed (the two legality guards:
  boundary spellings and the RAM-label `ImmLink` fixture), GREEN 8/8. No existing test asserted the old
  32-bit message.

### RF proof (invariant 8)

```
$ git status --short            (no tracked change)
$ (insert first body line of check_imm_ranges)
$ git diff --stat
 crates/sigil-frontend-emp/src/lower/code.rs | 1 +
 1 file changed, 1 insertion(+)
$ grep -n RF-MUTATION crates/sigil-frontend-emp/src/lower/code.rs
1410:    return Ok(()); // RF-MUTATION
$ cargo test -p sigil-frontend-emp --test imm_range
test byte_immediate_past_range_is_refused ... FAILED   (`move.b  #300, d1` assembled to [12, 3C, 00, 2C, 4E, 75]; expected a refusal)
test word_immediate_past_range_is_refused ... FAILED
test sr_and_memory_destinations_are_checked ... FAILED
test bit_numbers_are_checked ... FAILED
test result: FAILED. 4 passed; 4 failed
$ git restore --source=HEAD -- crates/sigil-frontend-emp/src/lower/code.rs
$ git status --short            (no tracked change)
$ cargo test -p sigil-frontend-emp --test imm_range
test result: ok. 8 passed; 0 failed
```
The 4 still green under the mutation are the two legality guards, the long-message test (that check is
in `m68k_operand`, not `check_imm_ranges`) and the const test (`const_arity.rs`).

Live-path probe: in a throwaway copy of the ref tree, `move.w #$12345, d0` and `btst #40, d0` added to
`engine/sound/sound_api.emp:115`, `SIGIL_CONTRACTS=0 sigil build`: candidate exit 1 with
`sound_api.emp:115:9: [Error] operand 74565 out of range -32768..=65535` and
`sound_api.emp:116:9: [Error] [lower.bit-number] bit number 40 out of range 0..=31`; base exit 0.

### Corpus and BN (candidate `sigil 0.1.0 (86272767)`, wall-clock 31.5 s)

Every shape built with exit 0 on the candidate, so 0 refusals in each of the 7 (the 584 sites grep
could not resolve are decided: none out of range).

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

### AG (throwaway aeon worktree at `936439fa`, `.target-land` `86272767`, removed after)

| Run | exit | wall-clock |
|---|---|---|
| `tools/landing_build.sh` | 0, `finished=0` (needs_build lane 35 ran / 0 failed / 1 exempted) | 547 s |
| `./build.sh` | 0 | 480 s |
| `DEBUG=1 ./build.sh` | 0 | 641 s |
| `./build.sh demo` | 0 | 658 s |
| `DEBUG=1 ./build.sh demo` | 0 | 483 s |

Every log names `sigil 862727673158`; `sigil`/`emit_sound_blob` md5 equal at start and end. A first
`landing_build.sh` attempt under a foreground `timeout 590` was killed at 590 s (machine shared with
other sessions) and is not counted; the counted run was detached with its own end marker.

### Tests (per crate, `--no-fail-fast`, `AEON_DIR` = ref tree)

| Scope | passed | failed | ignored |
|---|---|---|---|
| `sigil-frontend-emp` (139 binaries, 14 s) | 2,797 | 0 | 0 |
| `sigil-cli` (per test target, 1,396 s) | 850 | 57 | 1 |
| the other 12 crates (per crate; `sigil-harness` per target, `ORACLE_DIR` set; 566 s) | 2,182 | 2 | 2 |

All 59 failing names are in `.scratch-t1/failed_base.txt`; 0 new. Eight baseline names passed in this
per-crate environment (7 `sigil-cli` seam-2 slice tests and `aeon_dir_matches_the_provenance_tip`);
that is the environment, not this change. `frontend-emp` +1 binary, +8 passed vs Task 2.

### Booked, not in scope

- `move.w $FFFF8000, d0` accepted as an immediate (EMP finding 3, third bullet; 0 corpus hits by grep).
- (CLOSED in fix round 1, below.) The first cut read the encoder's `i32`, so `$80000000..=$FFFFFFFF`
  wrapped and passed.
- A scalar out of range NESTED in a const aggregate, or under a newtype, is still unchecked by
  `const_arity.rs` (emission owns it when the value is emitted).

### Fix round 1 (`1acf0518`, test fixture `HEAD`)

Review: the i32-wrap band still truncated silently (`move.w #4294967295, d0` `303C FFFF`,
`move.w #$FFFF8000, d0` `303C 8000`, `move.b #$FFFFFFFF, d0` `103C 00FF`, `const W: u32 = $FFFF9EDE` +
`move.w #W, d0` `303C 9EDE`; all exit 0, and the AS front end and asl (`range overflow`) refuse each).

Measured first: an instrumented build (uncommitted, `.target-instr`) printed every source immediate
outside `imm_bounds` that the encoder-side check accepted. Over the ref tree `936439fa`: plain 0, debug 0,
demo 0, demo_debug 0, config_a 0, config_b 0, lean 0, every CRC equal to base; a positive control
(`move.w #$FFFF8000, d0` added, `SIGIL_CONTRACTS=0`) printed 1. So the band is closed:
`check_code_imm_ranges` checks the i128 `CodeOperand` value before `m68k_operand` narrows it, on the
generic route (which covers imm-link, disp-sym, abs-sym and generic), the pc-relative route and
`encoded_len_m68k`. Messages now name the source value (`operand 4294934527`, `bit number 4294967295`).
The encoder-side `check_imm_ranges` stays as the seam backstop.

Tests: `wrapped_32_bit_values_in_byte_and_word_slots_are_refused` (RED at `86272767`, GREEN); `btst #9,
f(pc)` and `btst #-1, d0` added to `bit_numbers_are_checked`; the RAM fixture now asserts the lowered
module carries exactly `[(ImmWord16Be, 2)]` (the `.w` imm-link kind; `Value16Be` is the data kind and
rejects `$FFFF9EDE`, so it would not be the right assertion).

RF: `return Ok(()); // RF-MUTATION` as the first line of `check_code_imm_ranges` (code.rs:1421):
`wrapped_32_bit_values_in_byte_and_word_slots_are_refused` FAILED (8 passed / 1 failed); restored with
`git restore --source=HEAD`, 9/9.

One existing test changed input: `sigil-cli` `tranche20_spelling_probes`
`named_drain_label_jump_table_matches_as_arith_rept` wrote `move.w #QBASE` with `QBASE` a comptime `equ`
`$FFFF8000`, exactly the band, and failed `operand 4294934528 out of range -32768..=65535`. It now says
`#(QBASE & $FFFF)` (same bytes `31FC 8000 9000`, AS twin unchanged). The real `dma_queue.emp` site names a
RAM label (link-time, `ImmWord16Be`) and is unaffected.

BN (`sigil 0.1.0 (1acf0518)`, 33 s): `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`, CRCs as above.

AG (throwaway aeon worktree at `936439fa`, `.target-land` `1acf0518`, removed after; run detached
alongside the suites, so walls are inflated): `tools/landing_build.sh` exit 0, `finished=0`, stamp key
`3696a3447b0e95b0` (1,884 s); `./build.sh` 0 (614 s); `DEBUG=1 ./build.sh` 0 (476 s); `./build.sh demo`
0 (397 s); `DEBUG=1 ./build.sh demo` 0 (714 s). Every log names `sigil 1acf05185999`; binary md5 equal at
start and end.

Tests (per crate / per target, `--no-fail-fast`, `AEON_DIR` + `ORACLE_DIR`): `sigil-frontend-emp` 2,798 /
0 / 0 (139 binaries); `sigil-cli` 849 / 58 / 1 at `1acf0518`, the one new name being the tranche20 probe
above, 6/6 after its fixture change; other 12 crates 2,182 / 2 / 2. Failing names after the fixture
change: 59, all in `.scratch-t1/failed_base.txt`, 0 new.

## Task 4 (A8): CI green, and watched

Cause reproduced first (CI env, `.target-land` debug): `env -u AEON_DIR -u EMPYREAN_SUITE_ROOT
SIGIL_ALLOW_PARTIAL=1 cargo test -p sigil-cli --test test_g4_final_objects_port` gave 3 passed / 1
failed, `g4_objects_regions_match_reference` panicking at `test_g4_final_objects_port.rs:138`, `cannot
read /nonexistent/SIGIL_ALLOW_PARTIAL-no-reference-tree-was-named/games/sonic4/objects/test_player.emp`,
as CI run `36488380435`.

- Port test: one `REAL_FILES` list feeds every read and the guard
  `let Some(_) = reference_tree(&REAL_FILES) else { return };` at the top of `reference_gate`. It has 9
  entries, not the plan's 7: `player_block_ambient` also reads `games/sonic4/config/ram.emp` and
  `games/sonic4/player/player_common.emp`, and a guard that leaves out two of the reads can drift from
  them. The two other `compile_real_files` callers guard on the same list (`a6b6653a`). CI env: 4 passed
  with `skip:` lines. Strict, with the ref tree: 4 passed.
- `landing-run.sh` CI-SHAPED leg after the strict span: `env -u AEON_DIR -u EMPYREAN_SUITE_ROOT -u
  SIGIL_STRICT_GATE -u ORACLE_DIR -u SIGIL_BUILD -u SIGIL_EMIT SIGIL_ALLOW_PARTIAL=1 cargo test
  --no-fail-fast --workspace -- --nocapture`. It uses the debug profile, as CI does, and the same target
  dir. `CI_EXIT=` sits in its own `##### CI-SHAPED SPAN` and no strict-span reader looks inside it.
  `CI_RED` (non-zero exit, a failed test, or no `test result:` line) is part of the `RESULT FAILED`
  condition. The verdict line is `ci-shaped leg <rc>`, and `--verdict-only` refuses a log with no
  `CI_EXIT=`.
- `FAILING TESTS` comes from cargo's `failures:` blocks plus the `test ... FAILED` lines, one entry per
  (name, binary). On landing log `20261117T000459Z` the list said 63; it now says 67, the same multiset
  as `.scratch-t1/failed_base.txt`. The old list lost `mt_bank_debug_matches_reference` to an
  interleaved line, and `two_module_ownership_flip_debug`, red in 4 binaries, counted once. Fixture test
  `the_failing_list_names_every_test_cargo_lists_across_binaries`: 6 across two binaries reports 6.
- `docs/OVERSEER.md`: each loop runs `gh run list --branch master --limit 1 --json
  conclusion,headSha,createdAt`, and a `failure` stays the top row until fixed.
- Doc sync carried from Task 3, no byte change: `fixup.rs` `ImmWord16Be` no longer claims asl
  assembles the literal `move.w #$FFFF9EDE, d0` (asl refuses it, `range overflow`). The
  `lower_m68k_imm_link` doc now names `ImmWord16Be` for `.w`.

TDD: 5 new `landing_verdict.rs` tests. The stock fixture's CI span carries a `skip:` line, so every
older control also proves the strict readers stay out of it. RED against the old script: 14 passed /
20 failed. GREEN: 34/34.

### RF proof (invariant 8)

Committed baseline `a6b6653a`, `git status --short` empty. Mutation:
`sed -i '/let Some(_) = reference_tree(&REAL_FILES)/d' crates/sigil-cli/tests/test_g4_final_objects_port.rs`,
`git diff --stat` = `test_g4_final_objects_port.rs | 1 -` (the deleted line:
`-    let Some(_) = reference_tree(&REAL_FILES) else { return };`). Results with the mutation in place:

- the Step 1 command FAILED: 3 passed / 1 failed, `g4_objects_regions_match_reference`, `cannot read …/test_player.emp`;
- `scripts/landing-run.sh --scoped --aeon .aeon-sigil-ref-936439fa -- -p sigil-cli --test
  test_g4_final_objects_port` gave `CARGO_EXIT 0`, `CLIPPY_EXIT 0`, `LEDGER_EXIT 0`, `ci-shaped leg 101`,
  `CI-SHAPED FAILING TESTS (1) g4_objects_regions_match_reference (test_g4_final_objects_port)`, and
  `RESULT FAILED, the CI-SHAPED LEG is red (exit 101, 1 test(s) failed)`, exit 1.

Restored with `git restore --source=HEAD -- crates/sigil-cli/tests/test_g4_final_objects_port.rs`, status
empty. After the restore: Step 1 command 4 passed. The scoped landing with `--baseline 4` gave
`ci-shaped leg 0` and `RESULT GREEN`, exit 0.

### Totals

- Partial workspace, CI-shaped (`a6b6653a`, 253 s): 524 suites, 5,894 passed, 0 failed, 3 ignored, 457
  `skip:` lines, exit 0. The 67 golden-drift names (Task 6) skip in this shape.
- Full landing (`scripts/landing-run.sh --aeon .aeon-sigil-ref-936439fa`, `a6b6653a`, 600 s wall): strict
  span 524/524 binaries, 5,827 passed, 67 failed, 3 ignored, 0 skip lines, census 524. `FAILING TESTS
  (67)` is the same multiset as the baseline, so there are 0 new failures. Clippy 0, ledger 0,
  `ci-shaped leg 0 (524 suite(s), 5894 passed, 0 failed)`. RESULT FAILED on the 67 baseline names only.
- clippy `--workspace --all-targets -D warnings`, debug and release: 0 (local clippy 1.99.1).
- BN `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0` at `a6b6653a`, `6a0cd3ff` and `417dc819`.

### CI proof

Branch `parcel/step0-soundness` pushed (owner-approved; never master):

| run | head | conclusion | what |
|---|---|---|---|
| `44013571220` | `a6b6653a` | failure | test step: 524 suites, 5,893 passed, **1 failed**, 3 ignored, 475 skips. `g4_objects_regions_match_reference` ok. The red was `version_provenance` `every_closure_path_that_exists_is_watched_for_edits`: CI's `stable` is now rustc 1.100.0 (2026-11-09) and its new cargo build-dir layout moves the build script's recorded stdout (fixed `7d0aed5c`) |
| `44014834372` | `6a0cd3ff` | failure | test step GREEN (524, 5,894 passed, 0 failed, 3 ignored, 474 skips). clippy step red: clippy 1.100 `needless_range_loop` at `sigil-clownlzss-sys/src/accurate.rs:201,204` (fixed `417dc819`) |
| **`44015583371`** | `417dc819` | **success** | every step green: test (524 suites, 5,894 passed, 0 failed, 3 ignored), skip report (473), clippy, crate-graph guard (1 passed); 04:12:44Z to 04:17:28Z |

Master run after the merge: **`44017120477` at `457f2763`: success** (wave 1 landing, 2026-11-16; A8 task note updated). Booked: `docs/QUEUE.md` `CI-TOOLCHAIN-FLOATS`.

## Task 5 (A2): region sizes enforced, Z80 bank windows included

Commit `53484dfe` (sigil), paired with aeon `parcel/step0-soundness-pair` `ea39c166` + `0f7305cc`
(`games/sonic4/map.toml`: `[[region]] z80_sound_bank` gains `when = "sound_on"`; comment-only follow-up).
Before: a section could overflow a declared region and the build succeeded, three ways (Sigil audit
finding 8): `validate_section` held a section to the FIRST `rom` region holding its LMA (aeon's map lists
`rom` `[0,0x400000)` first, so nothing smaller was ever checked); `z80_bank` regions were read by nothing;
the seam-2 tails never ran a region check and their seven sizes were literals (`sound_tables_z80` 1,116 B
in `0x400`). Now:

- `MemoryMap::validate_section(name, placed_in, lma, len)`: (1) the region the section's placement group
  names (any kind; a group naming no region, the native `declchain{n}`, is ignored); (2) every `z80_bank`
  window holding `lma` must hold the end (a section starting at the window's end is outside it); (3) the
  first `rom` region holding `lma`. `object_bank` is a budget, never a container. Refusal text unchanged:
  ``section `N` [a,b) overflows region `R` (ends E), over by K bytes``.
- `LinkedSection.region` (= `Section.group`); `sigil_link::check_regions` is `emit_rom`'s loop; all seven
  seam-2 tails end in `seam2::check_placed_regions` (the map-placed sections only; the pinned carriers at
  `$1000000`+ are not the map's). Literal sizes replaced by `seam2::bank_room(lma) = 0x8000 - (lma & 0x7FFF)`.
- `native::check_budgets`: every `[[budget]]` row against its own `ceiling` (replaces
  `check_object_bank_budget` and the `"object_bank"` literal). A row with no region, no cursor, or a cursor
  no section defines is refused (`[map.budget-unmeasurable]`), not read as 0.
- `[[region]] when` (`PlacementMap.region_when`, `map_for_shape`): the native build checks the shape's map.
- Found on the way (`anchor_overlay.rs`, the clip-shaped overlay build, refused with ``section `dac_banks`
  [0xB8000,0xC649A) overflows region `z80_sound_bank` ``): an overlay moves the `sound_bank` anchor but not
  the region. `map_placement::move_bank_regions` moves a `z80_bank` region whose base is an anchor's `at` to
  that anchor's overlay address. Aeon's comment now says the equality is load-bearing.
- TDD. RED (stubs in place, signatures only): `map.rs` 4 failed / 9 passed (the boundary-accept tests
  pass: exact fill's first half, start-at-window-end, `object_bank` budget-not-container);
  `seam2_region_enforced` 6 failed / 2 passed; `native::budget_tests` 3 failed; `region_enforcement`
  3 failed / 2 passed (`nested_region_overflow_is_refused`: exit 0, the audit's z3). GREEN: all pass;
  `measure_real_sound_tables_z80_through_seam2 --ignored` at the ref tree: 1,116 B, head `0xB8000`,
  derived room 32,768.

### RF proof (invariant 8), two gates

```
(a) $ git status --short                 (no tracked change)
    $ git diff  crates/sigil-ir/src/map.rs  (3 deletions)
    -        for r in self.regions.iter().filter(|r| r.kind == RegionKind::Z80Bank && r.contains(lma)) {
    -            held(r)?;
    -        }
    $ cargo test -p sigil-ir --lib map::
    test map::tests::z80_bank_containment_refuses_a_section_crossing_the_window ... FAILED
      (called `Result::unwrap_err()` on an `Ok` value: ())
    test map::tests::z80_bank_exact_fill_is_accepted ... FAILED
    test result: FAILED. 12 passed; 2 failed
    $ git restore --source=HEAD -- crates/sigil-ir/src/map.rs; git status --short   (empty)
    test result: ok. 14 passed; 0 failed
(b) $ git diff  crates/sigil-link/src/lib.rs  (check_regions)
    -        map.validate_section(&s.name, s.region.as_deref(), s.lma, len)?;
    +        map.validate_section(&s.name, None, s.lma, len)?;
    $ cargo test -p sigil-ir --lib validate_placed     -> 2 passed (unit level is blind to this mutation)
    $ cargo test -p sigil-cli --test region_enforcement
    test nested_region_overflow_is_refused ... FAILED  (left: 0, right: 1: the nested overflow built)
    test result: FAILED. 4 passed; 1 failed
    $ git restore --source=HEAD -- crates/sigil-link/src/lib.rs; git status --short   (empty)
    test result: ok. 5 passed; 0 failed
```

### Whole-build red-first (reviewer B's e5 + e7)

Run-unique on-disk copy of the provisioned pair tree (ref `936439fa` + the pair `map.toml`), deleted after:
8 x `data _padxN = embed("games/sonic4/data/sound/song_s2_ooz.bin")` (2,876 B each, 23,008 B) appended to
`games/sonic4/data/sound/sfx_bank_blob.emp`, `song_bank_2` anchor `0xC0000` -> `0xC8000`.

- base `94cf811d`: **exit 0**, `built: sonic4 plain native ROM, crc=23ad4d05 len=883143` (the audit's e7 figure).
- candidate `53484dfe`: **exit 1**, no ROM written:
  ``error: native build (sonic4 plain): declared-chain: emit_rom: section `sfx_bank_blob` [0xBD5E8,0xC363C) overflows region `z80_sound_bank` (ends 0xC0000), over by 13884 bytes``.
  The plan predicted 13,888 (bank 1 "to `0xC3640`"); the section's last byte is `0xC363B`, the 4 B to
  `0xC3640` are the next section's alignment fill, not the section.

### Corpus and BN (candidate `sigil 0.1.0 (53484dfe)`)

0 refusals: every shape builds on the candidate. Pair tree (provisioned: ref `936439fa` + pair
`map.toml`; the pair branch differs from `936439fa` by docs + that file), 31.4 s, and the unpaired
control `.aeon-sigil-ref-936439fa`, 30.9 s, both:

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
Step 3 (aeon first, inert): base=cand=base over the pair tree, the same 7 rows, 7/7, rc 0 (104 s).

### AG (throwaway worktree of pair tip `0f7305cc`, binaries copied from `.target-land` `53484dfe`, removed after)

| Run | exit | wall-clock |
|---|---|---|
| `tools/landing_build.sh` | 0, `finished=0` (needs_build 35 ran / 0 failed / 1 exempted; land-gate stamp head `0f7305ccebe2`) | 591 s |
| `./build.sh` | 0 | 706 s |
| `DEBUG=1 ./build.sh` | 0 | 652 s |
| `./build.sh demo` | 0 | 413 s |
| `DEBUG=1 ./build.sh demo` | 0 | 383 s |

Every log names `sigil 53484dfed745`; `sigil`/`emit_sound_blob` md5 equal at start and end.

### SS (`scripts/landing-run.sh --aeon .aeon-sigil-ref-936439fa`, tree `53484dfe` clean, 633 s)

525/525 binaries launched and reported (census 525), **5,844 passed / 67 failed / 3 ignored**, 0 skip
lines; clippy 0, ledger 0, `ci-shaped leg 0 (525 suite(s), 5911 passed, 0 failed)`. `FAILING TESTS (67)`
sorted against `.scratch-t1/failed_base.txt`: `diff` empty, **0 new**; no log line carries an A2 refusal
text. Delta vs Task 4: +1 suite (`region_enforcement`), +17 passed (the new tests). RESULT FAILED on the
67 Task 6 baseline names only.

### Booked, not in scope

- `sigil build --check` stops before link, so it runs no region or budget check (its doc says so).
- `sigil emp --map` DOES reach `check_regions` (`link_rom` -> `emit_rom`, `crates/sigil-cli/src/main.rs:1825`);
  it has no shape, so it ignores a region's `when` and holds every region: it fails closed. (Corrected in
  fix round 1; the first cut of this line read as a gap.)

### Fix round 1 (`eab19132`): the bank window is tied to its anchor

Review (Important): a `z80_bank` region whose `lma_base` drifted from its anchor checked a window over
the wrong bytes and passed, with or without an overlay. `resolve_chained` now runs
`map_placement::bank_windows_for_shape` after `move_bank_regions` (which uses the same unique in-force
match): `[map.bank-window-unanchored]` when the base is no anchor's `at` (names the nearest in force), is
the `at` of 2+ in-force anchors, or its one anchor's `vma` differs from `vma_base`. A base that is only
the `at` of anchors NOT in force in the shape drops the window (it inherits its anchor's gate). The
literal "no in-force anchor -> refuse" was measured first and refused `config_b` on the unpaired control
(`936439fa`, no region `when`): ``z80_sound_bank` at 0xB8000 is at no anchor in force``, which would have
made sigil unable to build aeon master's sound-off shapes before the pair lands.
Live probe (pair tree, `sound_bank` anchor alone moved to `0xC8000`): exit 1,
``[map.bank-window-unanchored] z80_bank region `z80_sound_bank` at 0xB8000 is at no anchor; the nearest in force is `song_bank_2` at 0xC0000``.
`check_budget` also refuses a cursor below the region base and a ceiling past the region end (both
games' rows, ceiling `0x20000` = `object_bank` end, pass).

RF: `[] if pmap.anchors.iter().any(|a| a.at == r.lma_base) => continue,` mutated on disk to
`[] => continue, // RF-MUTATION` (`git diff --stat`: `map_placement.rs | 2 +-`) ->
`moving_the_anchor_alone_is_refused ... FAILED`, 17 passed / 1 failed; `git restore --source=HEAD`;
18 passed.

BN (`eab19132`): pair tree 7/7 rc 0 (30 s), control `936439fa` 7/7 rc 0 (31 s), the Task 5 rows. Per
test target, each under `timeout 590` (0 capped, 1,706 s total), `AEON_DIR` = control: sigil-ir 61/0,
sigil-link 173/0, sigil-harness 64 binaries 610 passed / 2 failed / 2 ignored, sigil-cli 186 binaries
855 / 57 / 1; all failing names are in `.scratch-t1/failed_base.txt` (0 new).

## Task 6 (A9): goldens at aeon 936439fa

Full account: `docs/superpowers/notes/2026-11-16-pin-advance-step0.md`. Commits `a8973000` (attest of the
`pin-advance-s2` tip, a precondition the freeze enforced: passed vs aeon 1ee78b88, 525 suites, 5,917 / 0 /
3), `fa042108` (freeze `pin-advance-step0`, chain len 208), `7dc3c1cf` (three port gates repaired by
derivation; the `module.path-mismatch` SITE_WATCH row for `mt_bank.emp` retired after aeon `97d139ca`).
`repin.toml`: no row changed. s4 `193507d9/850296` and s4.debug `ce326232/870560` equal aeon's figures;
demo, demo.debug, config_b unchanged; config_a `6be62db3/870972`, lean `60a3552b/798202`. No shipping byte
moved: the goldens now equal what the branch binary already produced at 936439fa (BN 7/7, rc 0).

SS: baseline at `503e4689` 5,850 / 67 / 3 (the 67 = `.scratch-t1/failed_base.txt`); after the freeze
5,914 / 3 / 3; after the repairs (`2ec622da`) RESULT GREEN, 525/525, 5,917 / 0 / 3, CI-shaped leg 0, BN 7/7.

## Task 7 (A5): Z80 code under the clobber gate

Commit `ffb0c18a` (sigil), paired with aeon `parcel/step0-soundness-pair` `eb634bbc` (27 x
`@allow("clobbers.computed-dispatch", "reached only through SeqOpcodeTable's ex (sp),hl dispatch")` in
`engine/sound/sound_sequencer.emp`: the 26 `Seq_Op_*` and `Seq_ContinueFetch`; declarations only).
Before: the Z80 transitive clobber closure ran only in `z80_clobbers_incomplete.rs` over the five
resident sound modules, reference-dependent; `sigil build` had no Z80 row at any warning level, so
reviewer B's e8 (`Audit_P () clobbers(a, f)` calling `Audit_Q () clobbers(a, b, f)`) built green.
Now:

- `sigil-frontend-emp/src/z80_closure.rs` (from `seam1.rs`): `z80_proc_node`, `z80_extern_node`,
  `z80_transfer_target`, `module_invariant_units`, `z80_clobber_partition`, and
  `z80_clobber_firings(procs) -> (firings, excluded)`. `ProcNode.computed_dispatch` is set from the
  proc's own `@allow("clobbers.computed-dispatch", …)`; the exclusion is by annotation and
  `is_opcode_dispatch_proc` is deleted. `first_carrier(name, …)` is used unchanged (via `check_firings`).
- The corpus Z80 pass (the one feeding `compute_verified_z80_outs`) builds the nodes from the same
  evaluated bodies. `ContractReport`: `z80_clobber_firings`, `z80_clobber_excluded`,
  `z80_clobber_unresolved`, `z80_dropped_instrs` (+ `_by_proc`), `z80_dropped_data_cells`.
- `run_contract_gate`: `[call.clobbers-incomplete] (Z80) closure firings` via `refuse_located` +
  `describe_clobber(f, "[call.clobbers-incomplete]")`; Z80 register-relevant drops and holes join the
  blindness check. `--report contracts`: `-- [call.clobbers-incomplete] (Z80) closure firings (n), m
  excluded by @allow("clobbers.computed-dispatch") --`, the rows, `EXCLUDED` lines, Z80 drop/hole counts.
- Found on the way: the corpus walk drops 10 `dc` cells in `DacSampleTable` (operands are
  `bankid`/`winptr` equs of link symbols; seam-1 resolves them with its banked carriers, the build's
  define set does not). `split_z80_drops` counts a dropped `dc` apart (it writes no register); only
  register-relevant drops are gated, 0 in every shape.

Step 1 (seam-1 harness at HEAD `1847c484`, `AEON_DIR=.aeon-sigil-ref-936439fa`, strict): 5/5 pass;
in-scope firings 0 both shapes; dispatch firings 66 on 17 of the 27 procs, both shapes; 0 dropped,
0 unresolved. The corpus walk agrees: ref `936439fa` 66 firings / 0 excluded in all 7 shapes; ref +
pair annotations 0 / 27 excluded in all 7. `Seq_Op_*` count from source: 26.

TDD: `z80_clobber_gate.rs` RED 2 failed (`the_build_gate_refuses…` at :95, `the_contract_report_prints…`
at :153) / 2 passed (the two negative controls); GREEN 4/4.

### RF proof (invariant 8)

```
$ git status --short                       (no tracked change, HEAD ffb0c18a)
$ git diff --stat
 crates/sigil-cli/src/main.rs | 2 +-
$ grep -n 'false && refuse_located("\[call.clobbers-incomplete\] (Z80)' crates/sigil-cli/src/main.rs
2277:    failed |= false && refuse_located("[call.clobbers-incomplete] (Z80) closure firings", &report.z80_clobber_firings, |f| f.span, |f| describe_clobber(f, "[call.clobbers-incomplete]"), &src_index);
$ cargo test -p sigil-cli --test z80_clobber_gate
test the_build_gate_refuses_a_z80_clobber_underclaim_with_its_location ... FAILED   (:95, the header assert)
test result: FAILED. 3 passed; 1 failed
$ git restore --source=HEAD -- crates/sigil-cli/src/main.rs; git status --short   (empty)
test result: ok. 4 passed; 0 failed
```

Whole build (e8), run-unique copy of the pair tree, `Audit_Q`/`Audit_P` appended to section
`sound_psg` (with one `nop` in `Audit_Q`; see below), each side's own `emit_sound_blob` first:
base `94cf811d` **exit 0** (`crc=017def01 len=850282`); candidate `ffb0c18a` **exit 1**:
``.../engine/sound/sound_psg.emp:734:9: [call.clobbers-incomplete] `Audit_P` destroys `b` through `Audit_Q`, which its clobbers(...) does not declare``.
Candidate with `SIGIL_CONTRACTS=0`: exit 0, the same `017def01`. Without the `nop` the resident blob is
odd-sized and BOTH binaries refuse the build (`[sound.bank-id-vs-placement] cannot measure: placed 6238,
emitted 6237`), a pre-existing behaviour, not this gate. Unannotated ref `936439fa`, candidate: exit 1, 66
located lines.

### BN

Aeon first (inert): base=cand=base `94cf811d` over ref `936439fa` + pair files, 7/7 rc 0 (30 s), the ref
rows. Base vs candidate `ffb0c18a` over the same pair tree, 7/7, `STEP0_CMP_END rc=0` (30 s):
plain 193507d9/850296, debug ce326232/870560, demo 2f9e2d4c/99994, demo_debug 64717bac/106981,
config_a 6be62db3/870972, config_b d888a312/617813, lean 60a3552b/798202.

### SS (`scripts/landing-run.sh --aeon .aeon-t7-pairref`, tree `ffb0c18a` clean, 550 s)

The reference is a detached aeon worktree at `936439fa` (the provenance tip) with the ref tree's
ignored artifacts and the two pair files applied (`DIRTY` by those two files only). RESULT GREEN:
526/526 launched and reported, **5,926 passed / 0 failed / 3 ignored**, 0 skip lines, clippy 0, ledger
0, CI-shaped leg 0 (526 suites, 5,926 passed). Delta vs Task 6 (525, 5,917): +1 suite
(`z80_clobber_gate`), +9 tests. A first run with the pair tree INSIDE the sigil checkout was RESULT
FAILED on 3 environment tests only (`installed_binary_needs_no_build_tree` x2: reference inside the
compiling checkout; `aeon_dir_matches_the_provenance_tip`: `git` read the sigil HEAD), discarded.

### AG (throwaway detached worktree of pair tip `eb634bbc`, binaries copied from `.target-land` `ffb0c18a`, removed after)

| Run | exit | wall-clock |
|---|---|---|
| `tools/landing_build.sh` | 0, `finished=0` (needs_build 35 ran / 0 failed / 1 exempted; land-gate stamp head `eb634bbc3017`) | 910 s |
| `./build.sh` | 0 (crc 193507d9 / 850,296) | 433 s |
| `DEBUG=1 ./build.sh` | 0 (ce326232 / 870,560) | 443 s |
| `./build.sh demo` | 0 (2f9e2d4c / 99,994) | 387 s |
| `DEBUG=1 ./build.sh demo` | 0 (64717bac / 106,981) | 369 s |

Every log names `sigil ffb0c18af4af`; `sigil`/`emit_sound_blob` md5 `0c4dc50b…`/`9c7f7679…` at the end
(copies, not rebuilt during the run). Ran concurrently with SS, hence the longer walls.

### Landing order (PAIR)

The candidate refuses any aeon tree without the annotations (66 located firings at `936439fa`, both
the builds and the strict suite's Z80 rows). Aeon `eb634bbc` must reach aeon origin/master before, or
with, the sigil merge; the standing ref tree then needs re-provisioning past it (QUEUE
`Z80-CLOBBER-BUILD-GATE`).

### Fix round 1 (`b436b12b`; aeon `168bb908`)

- I2: `[clobbers.computed-dispatch-reason-required]` (parser, same rule as `clobbers.unanalyzable`;
  `contract_grammar::computed_dispatch_allow_requires_reason`). Bound: `[clobbers.computed-dispatch-called]`
  refuses an annotated proc that an UNannotated proc `call`s/`rst`s. The "no direct call or jump" limit is
  disproved by the corpus: `Sequencer_NextOpcode`'s `jp z, Seq_Op_Ext` (`sound_sequencer.emp:1167`) is
  the one direct edge from outside the annotated set into it (5 more run between annotated procs). A jump is
  charged the target's full computed effect, so its declared set is relied on by no one along that edge;
  a call returns to a client that relies on it. Corpus: 0 firings, all 7 shapes.
- I3: chose (b), booked in QUEUE `Z80-CLOBBER-BUILD-GATE` with the two computed-transfer sites. W021 row
  corrected. (a) is not sound here: the table is named only in a comment, and spotting `ex (sp),hl; ret`
  as a dispatch from code is a heuristic.
- I4: one `  hint:` line under the Z80 row naming the firing procs with no direct caller in the Z80 corpus
  (derived from the closure's edges). Old aeon `936439fa`: 66 firings, hint names 15 handlers
  (`Seq_Op_Ext`, jumped to, and `Seq_ContinueFetch`, jr'd to, are not named). Artifact:
  `.aeon-v2-harness-out/sigil-t7/unannotated-ref-build.log` (exit 1, sigil `b436b12b`).
- I1 + the new rows, RF each (from `.aeon-v2-harness-out/sigil-t7/rf-fix1.txt`): mutation `if false && …`
  (or `failed |= false && …`), `git diff --stat` `main.rs | 2 +-`, red on the named test, restored with
  `git restore --source=HEAD`, 10/10 again:
  - `main.rs:2229` `if false && report.z80_dropped_instrs != 0 {` -> `a_register_relevant_z80_drop_is_refused_as_blindness` FAILED (:234), 9/1;
  - `main.rs:2240` `if false && !report.z80_clobber_unresolved.is_empty() {` -> `a_z80_hole_is_refused_as_blindness` FAILED (:250), 9/1;
  - `main.rs:2286` `failed |= false && refuse_located("[clobbers.computed-dispatch-called] firings"…` -> `an_annotated_proc_with_a_direct_caller_is_refused` FAILED (:278), 9/1;
  - `main.rs:2287` `failed |= false && refuse_located("Z80 name collisions"…` -> `a_z80_name_declared_twice_is_refused` FAILED (:318), 9/1;
  - `main.rs:2281` hint `if false && …` -> `a_firing_with_no_direct_caller_carries_the_dispatch_hint` FAILED (:267), 9/1;
  - `parser.rs:418` `if false && attr.args.len() != 2 || false && !has_reason {` -> `computed_dispatch_allow_requires_reason` FAILED (:375), 31/1; restored 32/32.
- Minors: M1 `dc` wording fixed (doc + QUEUE); M2 `Seq_ContinueFetch` reason re-worded in aeon
  `168bb908` (inert); M3 seam1 reads the invariant through `z80_closure::module_invariant_units`;
  M4 Z80 name collisions are a located zero-firing row; M5 module doc says the walk keys on the module
  `cpu`; M6 the 66-firing claim now has its artifact; M7 booked `SOUND-BANKID-ODD-RESIDENT-BLOB`.

## Task 8 (A4): one name environment per declaration

Commits (sigil): `7759b2d7` (step 1, the report, no behaviour change), `b35a2d38` (the 8 RED
fixtures), `a312c74a` (implementation), `85792b6b` (this note), `5fea0ad2` (fix after the first SS,
below); paired with aeon `3fd291f3` (the AG fix, below) on top of `parcel/step0-soundness-pair` `6b5f8d8d`
(`palette_dsl.emp` imports `PalCycleScript1/2`) and `c4c1fb0b` (EMP_PITFALLS §2/§8 retired;
DEFERRED_WORK `EMP-A4-PINS-REMOVABLE`).

Before: an imported comptime item was injected as a verbatim clone, so its free names resolved in
the IMPORTER (reviewer A probes: p8b `sizeof(S)` 12 not 6; p2d `9, 9` not `5, 5`; p2e 6 not 4; p8 an
error at `a.emp`). Now `resolve::hygiene` resolves each free name in the DEFINING module (own items,
private included, then its own `use`s), renames a comptime-bound one to `$def$name`, injects that
dependency under the hidden name (recursively, memoized per program), and maps a hidden dependency back
to its plain name in each importer that binds it to the same item. A name bound only by the importer is
refused: ``[name.unbound-at-definition] `X` in `f` is not bound in module `m`; add a `use` there``.

### Step 2: the corpus measurement (`sigil build --report name-env`, step-1 binary)

| Shape | rows (936439fa + A5 pair) | differ | unbound | rows (+ A4 pair) | differ | unbound |
|---|---|---|---|---|---|---|
| plain | 46,181 | 0 | 4 | 46,417 | 0 | 0 |
| debug | 51,045 | 0 | 4 | 51,307 | 0 | 0 |
| demo | 24,841 | 0 | 2 | 24,973 | 0 | 0 |
| demo_debug | 25,585 | 0 | 2 | 25,721 | 0 | 0 |
| config_a | 51,804 | 0 | 4 | 52,070 | 0 | 0 |
| config_b | 44,310 | 0 | 4 | 44,536 | 0 | 0 |
| lean | 46,181 | 0 | 4 | 46,417 | 0 | 0 |

The unbound rows (2 distinct): `engine.effects.palette_dsl.cycle_script1` names `PalCycleScript1` and
`cycle_script2` names `PalCycleScript2`, both defined in `engine.effects.palette`, bound only by the
importers `engine.effects.palette`, `games.sonic4.ojz_effects`, `games.sonic4.ojz_effects_editor_act1`.
The candidate binary's report over the pair tree is byte-identical to the step-1 binary's in all 7 shapes.

### RF (invariant 8), `.aeon-v2-harness-out/sigil-t8/rf.txt`

```
$ git status --short                     (empty, HEAD a312c74a)
$ git diff --stat
 crates/sigil-frontend-emp/src/resolve/hygiene.rs | 1 +
$ grep -n 'return vec!\[item.clone()\];' crates/sigil-frontend-emp/src/resolve/hygiene.rs
844:    return vec![item.clone()];
$ cargo test -p sigil-cli --test one_name_env
test struct_layout_is_computed_in_its_defining_module ... FAILED   left: [112, 12, 78, 117] (70 0C)
test result: FAILED. 1 passed; 7 failed
$ git restore --source=HEAD -- crates/sigil-frontend-emp/src/resolve/hygiene.rs; git status --short   (empty)
test result: ok. 8 passed; 0 failed
```

Second gate, the refusal: `hygiene.rs:900` `if false && memo.reported.insert(…)` →
`a_name_unbound_at_definition_is_refused` FAILED (`:238`), 7/1; restored, 8/8.

### BN (`.aeon-v2-harness-out/sigil-t8/bn1-*.log`, 34 s)

Base `94cf811d` vs candidate `a312c74a` over `.aeon-t8-pairref` (936439fa + the A5 pair files + the A4
`palette_dsl.emp`): **SHAPES 7/7 IDENTICAL, rc 0**; both columns equal the Task 6 goldens (plain
193507d9/850296, debug ce326232/870560, demo 2f9e2d4c/99994, demo_debug 64717bac/106981, config_a
6be62db3/870972, config_b d888a312/617813, lean 60a3552b/798202). The base column proves the aeon
`use` line byte-neutral under the shipping sigil. `--check` base vs candidate (plain, debug, demo):
identical ensure/LinkAssert counts (plain 89,905 / 698); the only difference is
`layout.provisional-drift` (4 plain, 5 debug) present under the base only: the base binary carries the
pre-Task-6 frozen tables (`offcanonical_sizes/s4*.txt` changed `94cf811d..68bfd8af`).

### First SS and AG were RED; fixed (`5fea0ad2`; aeon `3fd291f3`)

SS at `85792b6b`: RESULT FAILED, 6 tests. (1) Four pitcher-plant/tranche-0 example builds:
`[overlay.ambiguous-window] window `sst_custom` is ambiguous across $prelude$Sst.sst_custom,
Sst.sst_custom`: the prelude's own items were hidden at their definition and an importer binds a prelude
name as global, so they never mapped back. `localize` now maps a hidden PRELUDE item back when the
importer binds the name through the prelude; `resolve_bare_window` also skips `$`-named structs (not
names in scope). aeon has no prelude, so BN could not see this. (2) `sandbox_hermeticity`: the report
tests in `hygiene.rs` wrote files; moved to `tests/name_env_report.rs`. (3) `tranche7`'s
`imported_fn_referencing_home_private_const_fails_loudly` pinned the pre-A4 contract (F3's deep case
fails loudly); A4 closes it: now `..._resolves_where_defined`, `06 40 00 07 4E 75`.

AG at aeon `6b5f8d8d`: `tools/landing_build.sh` exit 1, `finished=1`, one tool test:
`test_citation_form.py::test_live_citations_resolve_to_something`, `docs/EFFECTS_AUTHORING.md:561 cites
engine/effects/palette_dsl.emp:32 -- blank line` (the reworded comment grew a line). Aeon `3fd291f3`
restores the comment's line count; DEFERRED_WORK cites `scene_registry.emp` by name.

Re-proved at `5fea0ad2`: RF both gates (`rf2.txt`, `rf2b.txt`: 70 0C red / 8/8; refusal 7/1 / 8/8);
report 0 differ / 0 unbound in all 7 shapes, byte-identical to the pre-fix report; BN base `94cf811d`
vs cand `5fea0ad2` over `.aeon-t8-pairref` (aeon content = pair `3fd291f3`) **7/7 IDENTICAL, rc 0**, 31 s,
the same rows as above.

### SS (`scripts/landing-run.sh --aeon .aeon-t8-pairref`, tree `5fea0ad2` clean, 663 s)

RESULT GREEN: 528/528 launched and reported, **5,947 passed / 0 failed / 3 ignored**, 0 skip lines,
clippy 0, ledger 0, CI-shaped leg 0 (528 suites, 5,947 passed). Delta vs Task 7 fix round (526, 5,933):
+2 suites (`one_name_env`, `name_env_report`), +14 tests (8 + 2 + 4 unit). The reference is a detached
aeon worktree at `936439fa` + the ref tree's ignored artifacts + the 3 pair files (`DIRTY` by those only).

### AG (throwaway detached worktree of pair tip `3fd291f3`, binaries copied from `.target-land` `5fea0ad2`, removed after)

| Run | exit | wall-clock |
|---|---|---|
| `tools/landing_build.sh` | 0, `finished=0` (needs_build 35 ran / 0 failed / 1 exempted; land-gate stamp head `3fd291f3dec9`) | 528 s |
| `./build.sh` | 0 (crc 193507d9 / 850,296) | 444 s |
| `DEBUG=1 ./build.sh` | 0 (ce326232 / 870,560) | 489 s |
| `./build.sh demo` | 0 (2f9e2d4c / 99,994) | 513 s |
| `DEBUG=1 ./build.sh demo` | 0 (64717bac / 106,981) | 517 s |

`sigil`/`emit_sound_blob` md5 `41886c8f…`/`7ddb94d3…` at the end (copies). Ran concurrently with SS.
Landing order: aeon `6b5f8d8d`, `c4c1fb0b`, `3fd291f3` (after A5's `eb634bbc`, `168bb908`) must reach aeon
master before, or with, the sigil merge; the candidate refuses a tree without `palette_dsl`'s import.

### Fix round 1 (`31367919`; aeon `6269c7fd`)

Review `task-8-review.md`: 0 Critical, 4 Important, 7 Minor. Every probe in it re-run on the candidate:
`equ` `70 03`, `equs` `70 03`, `unbeq` refused, `enumv` `08`, `ovl` and `ovlc` `4A29 0002`, `win` `70 08 4A29 0002`,
`win0` loud `[overlay.unknown-window]`, `diff` `70 03 72 08` (before: `70 09`, `70 09`, built, `15`, `$A`/error).

- I1: `is_comptime_kind` includes `Item::Equ` (`eval_path` reads an equ like a const). An equ dependency is
  injected as a hidden CONST: the definition-site fold where it folds, else its value with free names renamed.
  It emits no link symbol, so the `$def$name` spelling cannot collide with the equ's own mangled symbol
  (M6, noted on `hidden_name`). An importer equ now counts for `[name.unbound-at-definition]`, and
  `--report name-env` sees the class. Aeon pair `6269c7fd`: EMP_PITFALLS §2's "labels (procs, data, equs)
  resolve through the link" corrected, same line count (`tools/test_citation_form.py` 3 passed).
- I2: the walker no longer scopes variant names over discriminants and payload types.
- I3: a dotted window's struct (`region[0]`) is a walked name. A bare window the defining FILE cannot stamp
  (its struct is imported) is qualified in `HygieneMemo::prepare` to `Struct.window` with the one struct
  of the defining module's environment (own, `use`d, prelude) that has the field; 0 or 2+ candidates are
  left as written, for the defining module to report.
- I4 and M2: fixtures (below).
- Fixtures, `crates/sigil-cli/tests/one_name_env.rs` (8 -> 18): `a_private_equ_resolves_where_defined`,
  `a_private_equ_sizes_a_struct_where_defined`, `a_name_the_importer_binds_only_by_equ_is_refused`,
  `an_enum_discriminant_names_the_module_const_not_a_variant`, `a_dotted_overlay_window_resolves_where_defined`
  (with and without an importer `Sst`), `a_bare_overlay_window_resolves_where_defined` (with an unrelated
  importer struct, and with none), `a_hidden_struct_is_not_a_bare_window_candidate` (`win`),
  `a_hidden_struct_never_binds_a_bare_window_silently` (`win0`),
  `a_hidden_dependency_the_importer_also_imports_maps_back` (newtype `Px`: without map-back,
  `[cross-type mix] cannot mix Px and $a$Px`), `a_different_item_of_the_same_name_is_not_mapped_back` (`diff`).
  Unit: `free_names_sees_enum_discriminants_and_dotted_windows`.
- RF (`.aeon-v2-harness-out/sigil-t8/fr1/rf.sh`, log `fr1/rf.log`), runner
  `cargo test -p sigil-cli --test one_name_env`; each mutation shown by `git diff --stat` and `grep -n`, then
  `git restore --source=HEAD`:
  - I1 `hygiene.rs:552` `ast::Item::Const(_) /*RF-I1*/` (Equ dropped): 3 FAILED (`left: [112, 9, 78, 117]` x2, refusal not raised), 15/3;
  - I2 `hygiene.rs:424` scope extend re-added: `an_enum_discriminant…` FAILED `left: [21]`, 17/1;
  - I3 `hygiene.rs:462` `let _ = s;`: dotted and bare FAILED (`left: [74, 41, 0, 10, 78, 117]`), 16/2;
  - I3 `hygiene.rs:857` qualification removed: bare FAILED (`$A`), 17/1;
  - I4 `layout.rs:1086` filter deleted: `win` FAILED (false ambiguity) and `win0` FAILED (built silently), 16/2;
  - M2 `hygiene.rs:981` `false && o == origin`: map-back FAILED (cross-type mix), 17/1;
  - green at HEAD `31367919`: 18/18; `resolve::hygiene` unit 5/5.
- BN (`fr1/../bn3-*.log`, base `94cf811d` vs cand `31367919`, `.aeon-t8-pairref`): 7/7 IDENTICAL, rc 0, the
  golden CRCs. Name-env (cand `31367919`, all 7 shapes): 0 differ, 0 unbound; rows plain 46,428, debug 51,320,
  demo 24,973, demo_debug 25,721, config_a 52,083, config_b 44,547, lean 46,428 (+11/+13/0/0/+13/+11/+11
  vs Step 2: the equ and dotted-window rows now visible).
- SS (`scripts/landing-run.sh --aeon .aeon-t8-pairref`, tree `31367919` clean, 861 s): GREEN, 528/528,
  **5,958 passed / 0 failed / 3 ignored**, 0 skips, clippy 0, ledger 0, CI leg 0. Delta +11 (10 fixtures, 1 unit).
- AG (detached worktree of pair tip `6269c7fd`, binaries copied from `.target-land` `31367919`):
  `tools/landing_build.sh` exit 0, `finished=0` (35 ran / 0 failed / 1 exempted), 606 s; `./build.sh` 0
  (193507d9/850,296, 609 s); `DEBUG=1 ./build.sh` 0 (ce326232/870,560, 439 s); `./build.sh demo` 0
  (2f9e2d4c/99,994, 409 s); `DEBUG=1 ./build.sh demo` 0 (64717bac/106,981, 395 s). md5 `41bbb60b…`/`45cebf7a…`.
  Worktree removed.
- Left open: M1 (report rows top-level only) and M5 (param scoping order), booked in QUEUE `EMP-ONE-NAME-ENV`;
  M3 (the define half of report concern 2 is moot: `[defines.collision]` refuses an item named like a define);
  M4 (evidence, no change); M7 (process nits; `rf.sh` now prints the restore command).

## Task 9 (A6): front-end differential gate

Commits: `85dd7a05` (finding fix), `6fdcfbee` (gate), `9fe5b744` (Z80 count from tables; RF recorded).
Reference tree `.aeon-t9-pairref`: detached aeon worktree at `936439fa` (the provenance tip) plus the four
pair files from `6269c7fd` (`map.toml`, `sound_sequencer.emp`, `palette_dsl.emp`, `EMP_PITFALLS.md`),
uncommitted, ignored artifacts from `.aeon-t8-pairref`; `diff -rq` against `.aeon-t8-pairref` empty.
Artifacts: `.aeon-v2-harness-out/t9/`.

### The generator (`crates/sigil-cli/tests/support/frontend_diff.rs`)

- Families: `ALL_FAMILY_NAMES` minus the 7 that take no operand (`nop rts rte illegal reset rtr trapv`; the
  coverage test asserts the build never emits one with an operand). 67 vary both operands; 9 hold the
  destination (`move-to-sr/ccr/usp`, `andi/ori/eori-sr/ccr`), 2 hold the source (`move-from-sr/usp`).
- Sizes: none, `.b`, `.w`, `.l`, `.s` (`.s` and none added to the brief's three: the build emits `.s`
  branches, and unsized is how source spells `lea`/`moveq`/`btst`).
- Sources: 22 non-immediate (`Dn An (An) (An)+ -(An)`, `(d16,An)` at `i16::MIN/0/i16::MAX`, `(xxx).w` at
  `$0000/$7FFF/$8000/$FFFF`, `(xxx).l` at `0/$00FFFFFF/$FFFFFFFF`, `(d8,An,Xn)` at `i8::MIN/0/i8::MAX`,
  `(d16,PC)`, `(d8,PC,Xn)`, a register list, a label) plus `#imm` `{lo-1, lo, -1, 0, 1, hi, hi+1}` with
  `(lo, hi)` from `i8::MIN/u8::MAX`, `i16::MIN/u16::MAX`, `i32::MIN/u32::MAX` (unsized: the union, 15
  values; `.s`: the byte set). Destinations: each class once, and none (15).
- `Ea` gained `Idx PcDisp PcIdx RegList Label Sr Ccr Usp` beyond the brief's nine, because the s4 plain/debug
  encode probe (`t9/probe-triples.txt`) emits indexed, pc-relative, register-list, branch and sr/ccr operands.
- n = 67·153·15 + 9·153 + 2·5·15 = **155,292** (`expected_case_count`, from the tables).
- Z80: `ld r,n` (7 regs), `ld rr,nn` (6), `cp n`, 7 values each: 98 cases.

### Result (release, `9fe5b744`)

`TOTAL n=155292 same=4978 refuse=149345 excluded=969 disagree=0`; Z80 0 disagreements; 6,084 out-of-range
sized immediates, all `BothRefuse`. Per-family table: `t9/per-family.txt`. Debug-profile run (CI shape): 37 s.

The first run had 817 `Disagree` beyond Task 3's row. Every one was re-run through real asl 1.42 Bld 212
(`sonic_hack/tools/as/asl`, `t9/asl-verdicts.tsv`): the AS route matched asl's verdict and bytes on all of
them except two (`AS-MOVE-B-NEG-TO-CCR`). They split into:

- **Fixed, A1 class** (`85dd7a05`): 34 `.emp`-bytes cases, `andi/ori #imm, ccr` at `.w/.l/.s` truncated
  to the byte field (`andi.w #65535, ccr` -> `023C 00FF`, `ori.s #256, ccr` -> `003C 0000`); AS and asl
  refuse. `check_imm_ranges` now refuses `AndiCcr/OriCcr/EoriCcr` at any size but `.b`, the AS text.
  aeon spells all 20 ccr immediates `.b`. TDD: `imm_range.rs` `ccr_immediates_are_byte_only` RED then
  10/10.
- **Booked, not fixed**: `.emp` reads `usp` as a symbol (`EMP-USP-READ-AS-SYMBOL`: beside a proc named
  `usp`, `move.l a2, usp` assembles `21CA 0000`); `.emp` accepts `swap.b/.w/.s`.
- **Capability gaps** (`.emp` refuses, AS and asl assemble): `EMP-FRONTEND-DIFF-GAPS`.

EXCLUSIONS (each a named predicate on families, the exact `.emp` message class and which side refuses;
the two `.emp`-assembles rows also prove the bytes equal the AS route's legal spelling), hits:
`bit-number-past-width` 186, `emp-requires-a-size-suffix` 377, `emp-lacks-mnemonic` 161,
`emp-lacks-sr-ccr-form` 70, `emp-no-implicit-variant` 119, `emp-label-operand-gap` 45, `emp-lacks-usp` 2,
`branch-b-suffix` 3, `unsized-branch-aeon-pin` 3, `sized-swap` 3.

### Coverage (`crates/sigil-harness/tests/frontend_differential_coverage.rs`)

All 7 shipped shapes built under one `CaptureSession`: **481 emitted triples** (102,518 encodes, 59
families with operands) against **71,130 generated triples**: 0 missing. Single-test file (process-global
tap). Skips without the reference, panics under `SIGIL_STRICT_GATE=1`.

### RF (invariant 8; `t9/rf/rf1.sh`, `rf2.sh`, logs `rf1.log`, `rf2.log`)

```
RF 1  runner: CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-cli --test frontend_differential
--- HEAD 6fdcfbee branch parcel/step0-soundness; git status --short: (empty)
--- git diff --stat
 crates/sigil-frontend-emp/src/lower/code.rs | 2 ++
--- mutated lines
1422:    return Ok(()); // RF-MUTATION     (first line of check_code_imm_ranges)
1458:    return Ok(()); // RF-MUTATION     (first line of check_imm_ranges)
--- red run
533 of 6084 out-of-range immediates not refused by both front ends
TOTAL n=155292 same=5164 refuse=148718 excluded=783 disagree=627
  `move.b #256, d6`: emp bytes 4E711C3C00004E75 | AS refused ["operand 256 out of range -128..=255"]
EXCLUSIONS rows that match no disagreement: ["bit-number-past-width"]
test result: FAILED. 2 passed; 3 failed
--- restore: git restore --source=HEAD -- crates/sigil-frontend-emp/src/lower/code.rs; status (empty)
TOTAL n=155292 same=4978 refuse=149345 excluded=969 disagree=0
test result: ok. 5 passed; 0 failed

RF 2  runner: SIGIL_STRICT_GATE=1 AEON_DIR=.aeon-t9-pairref cargo test --release -p sigil-harness
      --test frontend_differential_coverage
--- git diff --stat
 crates/sigil-cli/tests/support/frontend_diff.rs | 8 ++++----
--- mutated lines (the four Ea::Disp rows commented out)
158:    // RF-MUTATION     Ea::Disp(i16::MIN, 1),
159:    // RF-MUTATION     Ea::Disp(0, 1),
160:    // RF-MUTATION     Ea::Disp(i16::MAX, 1),
191:    // RF-MUTATION     Some(Ea::Disp(8, 4)),
--- red run
frontend-differential coverage: 481 emitted triples (102518 encodes), 61695 generated triples, 82 missing
(lea, Some('l'), [(d16,An), An]) x705, e.g. in `sonic4 plain`   (first of 82 (d16,An) rows)
test result: FAILED. 0 passed; 1 failed
--- restore; status (empty); 71130 generated triples, 0 missing; test result: ok. 1 passed
```

### BN (`t9/bn.log`, base `94cf811d` vs cand `9fe5b744`, `.aeon-t9-pairref`, 74 s)

`SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`; rows equal the goldens (plain 193507d9/850296 ... lean
60a3552b/798202).

### First SS RED, fixed (`94802e92`)

`landing-run.sh --aeon .aeon-t9-pairref` at `9fe5b744`: RESULT FAILED, 3 red, all from the coverage file
(530/530 binaries, 5,962 passed, clippy 0, ledger 0): `source_gate_classification` x2 (the file reads the tree
but was in no run list; now in `scripts/nightly_source_gates.sh` beside `m68k_capstone_stream`) and
`skip_marker_lint` (the summary line said `missing`, skip vocabulary; now `not generated`). The run's one skip
line was that lint's own message quoting the marker.

### SS (`scripts/landing-run.sh --aeon .aeon-t9-pairref`, tree `94802e92` clean, 17:44:50Z -> 18:13:52Z)

**RESULT GREEN.** 530/530 binaries launched and reported; **5,965 passed / 0 failed / 3 ignored**, 0 skip
lines; CLIPPY 0, LEDGER 0; CI-shaped leg 0 (530 suites, 5,965 passed). Delta vs Task 8 fix round (528 /
5,958): +2 suites (`frontend_differential`, `frontend_differential_coverage`), +7 tests (5 + 1 + `imm_range`
`ccr_immediates_are_byte_only`). Log `.target-land/landing-20261117T174450Z.log`. AG not run: no aeon change,
and BN shows no byte moved.

### Fix round 1 (`f456fbc3`, `93c207fe`; review `task-9-review.md`: 0 Critical, 1 Important, 7 Minor)

- I1: `usp` reserved in `.emp` (`[operand.usp-unsupported]`, `[name.reserved-register]`); the differential's
  frames link against a defined `usp = $1234`. RED before the fix: `usp_reserved` 1/4 (`proc usp` + `move.l
  a2, usp` -> `4E75 21CA 0000 4E75`); the gate with the new frame `disagree=82` (`move.l a2, usp`: emp
  `21CA 1234`, AS `4E62`). After: 5/5, gate `disagree=0`, `emp-lacks-usp` 4 hits.
- M1: coverage also requires a SameBytes case per emitted triple: 480/481 at size, `(swap, .w, [Dn])` unsized.
- M2: four rows name closed family lists. M3: the `.emp` ccr text drops "or with no suffix".
- M4: `every_family_with_a_legal_emp_form_compared_bytes` (SameBytes > 0 except the 16 families derived from
  the exclusion lists, which must have 0) and 5 encoding canaries.
- RF (`.aeon-v2-harness-out/t9/rf/rf3.sh`, `rf3.log`; baseline `93c207fe`, restore `git restore --source=HEAD`):
  operand refusal off (`asm.rs:3256`) -> gate `disagree=84`, `move.l a2, usp` emp `21CA 1234` vs AS `4E62`,
  usp_reserved 2/3; declaration refusal off (`mod.rs:2641`) -> usp_reserved 2/3; both frames `BROKEN` ->
  `same=0`, floor and canaries FAILED; `(d16,An)` misspelled on both routes -> coverage `82 emitted triple(s)
  have no SameBytes case`. All green after restore.
- BN (`t9/bn3.log`, cand `93c207fe`): `SHAPES 7/7 IDENTICAL`, rc 0.
- SS (`landing-run.sh --aeon .aeon-t9-pairref`, tree `93c207fe` clean, 18:42:07Z -> 19:08:19Z): **RESULT GREEN**,
  531/531, **5,972 passed / 0 failed / 3 ignored**, 0 skips, clippy 0, ledger 0, CI leg 0 (531 suites, 5,972).
  Delta +1 suite (`usp_reserved`), +7 tests. Log `.target-land/landing-20261117T184207Z.log`.
- Left: M5 (asl byte table not committed as a fixture), M6 (round-0 ccr RED only in its commit message), M7
  (runtime figures vary with host load; fan-out mostly nominal).

### Fix round 2 (`a8dcdbd7`, `a6473695`; re-review `task-9-rereview-1.md`: I1, M1-M4 addressed, N1 Important)

- N1: a spliced label/string naming a register word is refused (`classify_operand_splice`), and a `Label`
  argument naming one is refused at the call (`check_arg_class`). Refusal only.
- N2: `usp_reserved` gains the `(usp).w/.l`, `dc.l/.w usp` and `usp+-k` refusals. N4: canaries must be
  generated cases. N3 left (unsized fallback not bounded).
- RF (`.aeon-v2-harness-out/t9/rf/rf4.sh`, `rf4.log`; baseline `a6473695`, restore `git restore --source=HEAD`):
  splice guard off -> `register_word_splice` 2/5, `21CA 1234 4E75` and Z80 `21 3412 C9`; Label check off ->
  1 fails (the splice still refuses, so the located call-site error is what goes); both off -> 1/5, the
  reviewer's `21CA 1234`; dc refusal off -> `usp_reserved` 5/6 (`dc.l usp` assembled `0000 1234`); a canary
  off the matrix -> `moveq #2, d6 is not a generated case`. All green after restore.
- Gate: TOTAL n=155292 same=4978 refuse=149345 excluded=969 disagree=0, 7/7. Coverage 480/481 + swap unsized.
  `sigil-frontend-emp`: 140 suites, 2,822 passed, 0 failed.
- BN (`t9/bn4.log`, cand `a6473695`): `SHAPES 7/7 IDENTICAL`, rc 0.
- SS (`landing-run.sh --aeon .aeon-t9-pairref`, tree `a6473695` clean, 19:29:49Z -> 20:02:42Z): **RESULT GREEN**, 532/532, **5,978 passed / 0 failed / 3 ignored**, 0 skips, clippy 0, ledger 0, CI leg 0 (532 suites, 5,978). Delta +1 suite (`register_word_splice`), +6 tests. Log `.target-land/landing-20261117T192949Z.log`.
- Booked: `EMP-REGISTER-NAMES-CASE-SENSITIVE` (latent; aeon count 0).

## Wave 3 landing checklist (final review I4; order is load-bearing)

Sigil at wave 3 (A5, A4, A6 and the fix wave) refuses every aeon tree that lacks the pair
`parcel/step0-soundness-pair` (A5's Z80 annotations, A4's `use` lines), measured by the final review on
unpaired `936439fa`: 66 located `[call.clobbers-incomplete] (Z80)` firings. Four consumers read different
refs, so the order below is what keeps each of them green. Do not reorder.

1. **Aeon local master merges the pair FIRST**, before sigil local master receives wave 3.
   `sigil-source-gates.timer` (05:17) reads LOCAL `master` of both repos (`nightly_source_gates.sh`); a
   sigil master with wave 3 beside an aeon master without the pair fails that night. The final review
   checked `git merge-tree master 6269c7fd` in aeon: clean.
2. **Merge, not squash or cherry-pick**, for the aeon pair (merge or fast-forward). The Task 6 attest and
   the review trail name pair SHAs; a squash makes them point at commits master never has. (The Z80
   remedy hint no longer names a SHA, fix wave, so it no longer depends on this.)
3. **AG on that merge** (the plan's "re-run AG with the shared pair"): `tools/landing_build.sh` with its
   `finished=` stamp, then the four `build.sh` shapes, `SIGIL_BUILD`/`SIGIL_EMIT` = this branch's build.
   Aeon master has moved `build.sh` since the pair's AG runs (190 lines), so the pair's own AG does not
   stand in for this one.
4. **Push aeon** (owner approval). Aeon local master is well ahead of origin with other plans' work;
   pushing the pair means pushing those too, or cherry-picking, which breaks rule 2. Owner call.
5. **Then sigil wave 3**: merge `parcel/step0-soundness` into sigil local master, CI green at the final
   tip (the pinned workflow has never run; record the A6 runtime on CI's runner, final review M7), push.
   `sigil-ref-drift.timer` (07:17) reads ORIGIN master of both repos: sigil origin with wave 3 before aeon
   origin with the pair exits 2 (NOTHING MEASURED). Steps 4 then 5 keep it measuring.
6. **Do not rebuild the shared `sigil/target/release/sigil` from wave-3 master before aeon master has the
   pair** (step 1). `aeon-effects-gates.timer` (`nightly_effects_gates.sh`) and every aeon `build.sh` run
   without `SIGIL_BUILD` use that binary; rebuilt early, it refuses aeon master.
7. **The goldens pin and the standing reference.** The pin stays at `936439fa` (provenance #208), and a
   provisioned `936439fa` tree is red on the Z80 rows by design. `provision-aeon-ref.sh` refuses
   non-origin commits, so until the pair is on aeon origin every SS runs against the hand-made overlay
   `.aeon-t9-pairref` (`936439fa` + the pair's source files as working-tree changes), which is not
   reproducible from a SHA. After step 4: pin-advance to the pushed aeon revision, provision
   `.aeon-sigil-ref-<rev>`, update `docs/OVERSEER-REFERENCE.md`'s standing-reference paragraph, and retire
   `.aeon-t9-pairref`, `.aeon-sigil-ref-936439fa-t6` and (when no lane builds a pre-`97d139ca` tree)
   `.aeon-sigil-ref-1ee78b88`.
8. Record `merge-base --is-ancestor` of each tip in each master (invariant 12), and the A-task notes
   (`status`, `branch`, one Progress line with SHA) in empyrean.

