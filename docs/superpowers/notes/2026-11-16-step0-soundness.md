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
| `44013571220` | `a6b6653a` | failure | test step: 524 suites, 5,893 passed, **1 failed**, 3 ignored, 475 skips. `g4_objects_regions_match_reference` ok. The red was `version_provenance` `every_closure_path_that_exists_is_watched_for_edits`: CI's `stable` is now rustc 1.100.0 (2026-11-09) and its new cargo build-dir layout moves the build script's recorded stdout (fixed `6a0cd3ff`) |
| `44014834372` | `6a0cd3ff` | failure | test step GREEN (524, 5,894 passed, 0 failed, 3 ignored, 474 skips). clippy step red: clippy 1.100 `needless_range_loop` at `sigil-clownlzss-sys/src/accurate.rs:201,204` (fixed `417dc819`) |
| **`44015583371`** | `417dc819` | **success** | every step green: test (524 suites, 5,894 passed, 0 failed, 3 ignored), skip report (473), clippy, crate-graph guard (1 passed); 04:12:44Z to 04:17:28Z |

Master run after the merge: **MASTER_RUN_ID_PLACEHOLDER, recorded by the controller at landing** (also owed: the A8 task note `empyrean/docs/v2/tasks/A8 CI green and watched.md`, status/branch/Progress with both run ids). Booked: `docs/QUEUE.md` `CI-TOOLCHAIN-FLOATS`.
