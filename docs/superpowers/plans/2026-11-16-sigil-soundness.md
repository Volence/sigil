---
kind: plan
status: awaiting-owner
description: "Empyrean v2 step 0, tasks A1-A9: close Sigil's two wrong-ROM paths (immediates, region sizes), make every computed check gate the build (incl. Z80 and a located error), one name environment per declaration, a front-end differential gate, CI green and watched, goldens at aeon HEAD."
---
# Sigil Soundness (Empyrean v2 step 0, A1-A9) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a green `sigil build` trustworthy: no silent truncation, no silent region overrun, every computed check family stops the build with a location, one meaning per declaration, an independent oracle on the `.emp` front end, CI green, goldens at aeon HEAD.

**Architecture:** Every fix extends an analysis that already exists (no rewrite). Front-end checks live in `sigil-frontend-emp`; region checks in `sigil-ir`/`sigil-link` with the seam-2 sizes derived in `sigil-harness`; gating lives in `sigil-cli`'s `run_contract_gate`. No task is byte-changing: each proves 7/7 shapes byte-identical at aeon HEAD. The two aeon changes (A2 map key, A5 annotations) are declarations that emit nothing and land FIRST, inert under the shipping binary.

**Tech Stack:** Rust workspace (`sigil-*` crates), `.emp`/AS sources, aeon `build.sh` / `tools/landing_build.sh`, sigil `scripts/landing-run.sh`, `scripts/provision-aeon-ref.sh`, `refreeze`/`repin`, GitHub Actions (`.github/workflows/ci.yml`).

**Spec:** `empyrean/docs/v2/Empyrean v2 architecture.md` (step 0 row, G0 gate) and `empyrean/docs/v2/tasks/A1..A9*.md`. Evidence: `sigil/docs/audits/EMP language.md` (findings 1, 2, 3) and `sigil/docs/audits/Sigil assembler.md` (findings 1, 2, 3, 4, 8) with their stage files.

## Global Constraints

- G0 exit (verbatim): "CI green; both wrong-ROM paths closed; checks gate".
- Not byte-changing. Every task ends with `scripts/step0_shapes_cmp.sh` (Task 0) printing `SHAPES 7/7 IDENTICAL` at aeon HEAD. A task that moves a byte STOPS: BLOCKED, owner call.
- One sigil branch: `parcel/step0-soundness`, cut from sigil master `94cf811d`. One aeon pair branch: `parcel/step0-soundness-pair`, cut from aeon master at execution time (today `90a45adb`). Parallel tasks may use worktrees on `parcel/step0-soundness--aN` merged back into the branch; never commit to master.
- Paired landing (aeon `CLAUDE.md`, invariant 7): aeon and sigil changes that depend on each other merge as a PAIR; any cross-seam change runs sigil's full suite with `--no-fail-fast`.
- Build env: `SIGIL_BUILD` / `SIGIL_EMIT` must point at THIS branch's own release build (`CARGO_TARGET_DIR=.target-land`), never the shared `sigil/target/release` (finding: aeon `build.sh:652-660`, a shared binary once three days stale). Missing env = BLOCKED, not a workaround.
- Aeon green = `tools/landing_build.sh` exit 0 with its `finished=<n>` stamp, plus `./build.sh`, `DEBUG=1 ./build.sh`, `./build.sh demo`, `DEBUG=1 ./build.sh demo` (invariant 6).
- Sigil green = `scripts/landing-run.sh --aeon <provisioned ref tree>`: `RESULT GREEN`, aggregate totals quoted (never a tail).
- Gates follow invariant 8 via protocol **RF** below. No gate may render "could not measure" as green.
- Never use `mcp__oracle__*` tools. Headless gates that start their own process are allowed.
- Exact-path `git add`; verify each commit with `git show --stat`; write the finding into the commit message (invariant 11).
- Standing ruling (`main.rs:2138-2140`): never edit engine CODE to please a checker. A5's annotations are declarations, not code.
- Do not pin aeon ROM sizes in docs; goldens/provenance are the only place sizes are recorded.
- Doc sync in the same change: `docs/QUEUE.md` rows closed or booked, the parcel note `docs/superpowers/notes/2026-11-16-step0-soundness.md`, and each `empyrean/docs/v2/tasks/A<n> *.md` (`status`, `branch`, one Progress line with SHA) at landing.
- `sigil build` (even `--check`) rewrites 15 files under `aeon/engine/sound/generated/` (Sigil audit finding 9): never run it on a live aeon tree; build in a run-unique copy.

## Review Focus

1. **Link-time word immediates holding RAM addresses** (`move.w #RamLabel, d0`, label at `$FFFF9EDE`) must keep assembling (`ImmWord16Be` accepts a sign-extended high half, `sigil-link/src/lib.rs:752-775`). A1's check must not touch the `ImmLink` path. Test in Task 3.
2. **Boundary and complemented spellings**: `#-128`, `#255`, `#-32768`, `#$FFFF`, `andi.b #~$7F` are legal; `#-129`, `andi.b #~$80`, `move.w #$12345, sr`, `andi.b #$1FF, ccr` are not. The corpus uses `#~CONST` in `.b` at 10 sites. Test in Task 3.
3. **A region not in force in a shape** (the `z80_sound_bank` window in the sound-off shapes, where ordinary code may one day flow through `$B8000-$BFFFF`) must not be enforced there. Test in Task 5.
4. **Exact fill and start-at-end**: a section that fills a Z80 window exactly, and `SongBank2_Head` starting at exactly `$C0000`, are legal; a region that is a budget, not a container (`object_bank`, with `OJZ_Sec8_Blocks` at `$1FF86` running past `$20000` today), must not be treated as a container. Tests in Task 5.
5. **Transitive and private helper chains**: a struct whose field type is a struct from a third module, and a `pub comptime fn` whose private helper calls another private helper, must resolve wholly in their defining modules; an importer's own same-named items must still mean themselves in its own code. Tests in Task 8.

## Baseline measured for this plan (2026-11-16)

All with the shipping `sigil 0.1.0 (7630ba0d)` (its closure revision is unchanged at `94cf811d`), over a `git archive` copy of aeon `90a45adb` in the scratchpad, plain (non-DEBUG) shapes unless stated. DEBUG shapes need the gitignored `engine/debug/generated/*` artifacts, so a bare `git archive` cannot build them (`[embed.not-found] engine/debug/generated/s4lz_plain.bin`); Task 0 provisions instead.

| Item | Result | How |
|---|---|---|
| Re-verified citations | all hold at `94cf811d`: `lower/code.rs:1396-1405`, `m68k.rs:13-21,1907-1913`, `frontend-as/src/eval.rs:3333-3338`, `map.rs:99-101,135-136`, `native.rs:4479`, `main.rs:2153-2375,2236-2248,2525-2575`, `seam1.rs:1030-1060`, `test_g4_final_objects_port.rs:138,417-433`; `winptr` mask is `eval/builtins.rs:457-476` (the audit's `sigil-link/src/lib.rs:1594` is a test of it) | read |
| A1 probes | `move.w #$12345` `303C 2345`; `move.b #300` `123C 002C`; `andi.b #$1FF` `0200 00FF`; `cmpi.b #300` `0C00 002C`; `move.b #-129` `143C 007F`; `move.w #$12345, sr` `46FC 2345`; `andi.b #$1FF, ccr` `023C 00FF`; `addi.w #-32769` `0640 7FFF`; `btst #40, d0` `0800 0028`; all exit 0. AS route refuses the first five with `operand N out of range lo..=hi` and accepts `btst #40` | probes |
| A1 corpus | sized `.b`/`.w` immediates: 0 of 879 resolvable sites out of range (566 not resolvable by grep); the 10 `.b #~X` sites all in range; bit numbers: 0 of 209 resolvable (18 not); typed scalar consts with a value: 0 | grep + s4 listing EQUs |
| A2 probes | `z80probe` z2 (7 B section in a 4 B region) exit 1; z3 (same, wider `rom` region listed first) exit 0, byte-identical to z1. Bank 1 at HEAD: `$B8000-$BDC5C`, 9,124 B headroom before `SongBank2_Head` `$C0000`; `config_b` 617,813 B ends below `$B8000`; a `when` key on `[[region]]` is silently accepted by today's map loader | probes, s4 listing |
| A3 corpus | `[bus.*]`, `[branch.condition-constant]`, `[call.slot-type-mismatch]/[option.unguarded-use]`: 0 firings in all 7 shapes; `[proc.clobber-undeclared]` closure: 0 | `sigil build --report contracts` x7 |
| A4 probes | p8b `sizeof(S)` = 12 (want 6); p2d `9, 9` (want 5, 5); p2e `6` (want 4) | probes, `--root` |
| A4 corpus | token-level probe: 0 imported struct/fn free names whose importer binding differs from the definition's (2 hits, both the same canonical struct) | script, ESTIMATED |
| A5 corpus | exclusion set by grep: 27 procs (26 `Seq_Op_*` + `Seq_ContinueFetch`, all in `engine/sound/sound_sequencer.emp`); `@allow("clobbers.computed-dispatch", …)` builds exit 0 today | grep, probe |
| A8 cause | CI red since merge `b4472d9c` (2026-09-28T18:11Z, `parcel/pin-advance-s2`); last 12+ master runs red on ONE test, `g4_objects_regions_match_reference`, panicking at `test_g4_final_objects_port.rs:138` reading `.../games/sonic4/objects/test_player.emp` with no reference guard | `gh run list`, `gh run view --log-failed` |
| A9 drift | goldens pin aeon `1ee78b88`; aeon is 64 commits ahead. At `90a45adb`: s4 `193507d9`/850,296 B (sha256 `1d679068eb1180ff…` = live `aeon/s4.bin`), demo `2f9e2d4c`/99,994 and config_b `d888a312`/617,813 equal the goldens, lean `60a3552b`/798,202 moved | zlib CRC-32 |

## Order, dependencies, parallelism

| Order | Task | Spec id | Depends on | Parallel with | Aeon change |
|---|---|---|---|---|---|
| 0 | Baseline + byte-neutral harness | — | — | — | none |
| 1 | Printed families gate | A3 | 0 | 3, 4 | none (0 firings) |
| 2 | Located clobber error | A7 | 1 (same function) | 3, 4 | none |
| 3 | Immediate range checks | A1 | 0 | 1, 2, 4 | none expected |
| 4 | CI green and watched | A8 | 0 | 1, 2, 3 | none |
| 5 | Region sizes enforced | A2 | 0 | 6, 7, 8 | `map.toml` `when` key, lands first |
| 6 | Goldens at aeon HEAD | A9 | 4 (CI trusted) | 5, 7, 8, 9 | none |
| 7 | Z80 under the clobber gate | A5 | 2 (located render) | 5, 6, 8, 9 | 27 annotations, land first |
| 8 | One name environment | A4 | 6 (HEAD goldens as byte guard) | 5, 7, 9 | only if its report finds unbound names |
| 9 | Front-end differential gate | A6 | 3 | 5, 6, 7, 8 | none |

Tasks 1-4 are the cheap independent wins (hours to a day each). 1 and 2 edit the same function, so they run in sequence.

## Shared protocols

**RF (red-first, invariant 8).** For each gate a task adds:
1. Commit the gate and the fix; `git status --short` shows no tracked change.
2. Apply the task's named mutation with the given command. Show `git diff --stat` naming the file and quote the mutated line back with `grep -n`.
3. Run the named test. It must FAIL on the named assertion. Mutation applied and still green = runner defect (stale binary, wrong target dir): fix the runner before claiming the gate.
4. Restore with `git restore --source=HEAD -- <file>` (only that file was dirty); `git status --short` empty; re-run → PASS.
5. Paste steps 2-4 output into the parcel note and the next commit message. If the method changes mid-parcel, re-prove every earlier gate that only printed `ok` (8e).

**BN (byte-neutral).** `scripts/step0_shapes_cmp.sh --base .target-base/release/sigil --cand .target-land/release/sigil --aeon-ref <tree>` → `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.

**AG (aeon green).** In the aeon pair worktree (or a master worktree when the task has no aeon change), with `SIGIL_BUILD`/`SIGIL_EMIT` = this branch's `.target-land` build: `tools/landing_build.sh` exit 0 with `finished=` stamp; then the four `build.sh` shapes exit 0.

**SS (sigil suite).** `scripts/landing-run.sh --aeon <ref tree>` → `RESULT GREEN`; quote launched/reported binary counts and pass/fail/ignored totals.

---

### Task 0: Branch, baseline, byte-neutral harness

**Files:**
- Create: `scripts/step0_shapes_cmp.sh`
- Create: `docs/superpowers/notes/2026-11-16-step0-soundness.md`

**Interfaces:**
- Produces: `scripts/step0_shapes_cmp.sh --base <sigil> --cand <sigil> --aeon-ref <dir>`; exit 0 identical, 1 differ, 2 could not run; prints one `SHAPE <name> <crc32>/<size> <crc32>/<size> SAME|DIFFER` row per shape, then `SHAPES <k>/7 IDENTICAL`, then `STEP0_CMP_END rc=<n>`. Shapes, derived from `sigil build --help`: plain, `--debug`, `--game demo`, `--game demo --debug`, `--config-a`, `--config-b`, `--lean`.
- Produces: `.target-base/release/sigil` (master `94cf811d` build) and the provisioned tree `/home/volence/sonic_hacks/.aeon-sigil-ref-<aeon rev8>`.

- [ ] **Step 1: Branch and base build.** `git switch -c parcel/step0-soundness` from `94cf811d` (in a worktree). Build the base from a detached `94cf811d` worktree: `CARGO_TARGET_DIR=.target-base cargo build --release --locked -p sigil-cli -p sigil-harness --bins`. `sigil --version` must print `closure-revision: 07f9caaa…`, same as the shipping binary.
- [ ] **Step 2: Provision aeon HEAD.** `REF_BUILD_DEMO=1 SIGIL_BUILD=.target-base/release/sigil SIGIL_EMIT=.target-base/release/emit_sound_blob scripts/provision-aeon-ref.sh /home/volence/sonic_hacks/.aeon-sigil-ref-<rev8> <aeon HEAD>`; expect `PROVISION_END rc=0`.
- [ ] **Step 3: Write the script.** It `cp -a`s the reference tree to a run-unique scratch dir per binary (the build writes into the tree), builds each shape with `-o`, CRCs with python `zlib.crc32`, compares, and exits 2 with `COULD NOT RUN <shape>: <reason>` if any build fails or a file is missing (never a pass).
- [ ] **Step 4: RF for the script.** Mutation: a `--cand` wrapper script that runs the real binary and then flips byte `0x200` of its `-o` file. Expected `SHAPES 0/7 IDENTICAL`, rc 1. Then base against base: `SHAPES 7/7 IDENTICAL`, rc 0.
- [ ] **Step 5: Record the baseline.** Paste the seven CRC/size rows and the Baseline table above into the parcel note. At `90a45adb` the plain rows must read s4 `193507d9/850296`, demo `2f9e2d4c/99994`, config_b `d888a312/617813`, lean `60a3552b/798202`.
- [ ] **Step 6: Commit** `scripts/step0_shapes_cmp.sh docs/superpowers/notes/2026-11-16-step0-soundness.md`.

---

### Task 1 (A3): The three printed check families gate the build

**Files:**
- Modify: `crates/sigil-cli/src/main.rs:2233-2245` (the `empty_gate` block in `run_contract_gate`)
- Create: `crates/sigil-cli/tests/printed_families_gate.rs`
- Modify: `docs/QUEUE.md` / ledger row carrying `[bus.*]` as "reported, not gated" (close it)

**Interfaces:**
- Produces (used by Tasks 2 and 7), in `main.rs`:
  `fn refuse_located<T>(header: &str, firings: &[T], span_of: impl Fn(&T) -> Option<sigil_span::Span>, describe: impl Fn(&T) -> String, src_index: &SourceIndex) -> bool`
  Prints `error: {header}, {n} firing(s); this family is zero-firing by contract:` then one `  {file:line:col}: {describe}` per firing (`  (no span recorded): {describe}` when `None`); returns `n != 0`.
- Describe strings (the codes are the existing ones at `main.rs:2533,2570-2574`):
  - bus: ``{code} in `{proc}` `` with code in `[bus.double-stop]`, `[bus.start-without-stop]`, `[bus.stopped-at-return]`, `[bus.vdp-write-unstopped]`, `[bus.released-at-return]`
  - branch: ``[branch.condition-constant] `{proc}` b{cc} is statically decided ({ALWAYS|NEVER} taken)``
  - slot: ``{id} `{proc}` calls `{callee}`: slot {reg} expects {expected} but found {found}{remedy}`` (the report's text)
- Headers: `[bus.*] Z80-bus machine-state firings`, `[branch.condition-constant] firings`, `[call.slot-type-mismatch]/[option.unguarded-use] firings`.

- [ ] **Step 1: Write the failing test** (fixture-tree pattern of `crates/sigil-cli/tests/flag_result_gate.rs:181-233`; its header explains why the exit code carries no signal and only text is asserted).

```rust
// fixture: BusP (bus fixture of frontend-emp/tests/z80_bus.rs:83-93),
// BranchP (branch_const.rs:30-44), SwapP calls TakeX with a GridY in d2 (type_slice.rs PRE)
#[test]
fn the_build_gate_refuses_each_printed_family_with_its_location() {
    let err = build_stderr(tree(FIRING), false);
    assert!(err.contains("error: [bus.*] Z80-bus machine-state firings, 1 firing(s); this family is zero-firing by contract:"));
    assert!(err.contains(&format!("  {}: [bus.vdp-write-unstopped] in `BusP`", loc(&f, FIRING, "move.w  d0, VDP_CTRL"))));
    assert!(err.contains("error: [branch.condition-constant] firings, 1 firing(s); this family is zero-firing by contract:"));
    assert!(err.contains(&format!("  {}: [branch.condition-constant] `BranchP` bne is statically decided (NEVER taken)", loc(&f, FIRING, "bne     .spin"))));
    assert!(err.contains("error: [call.slot-type-mismatch]/[option.unguarded-use] firings, 1 firing(s); this family is zero-firing by contract:"));
    assert!(err.contains("[call.slot-type-mismatch] `SwapP` calls `TakeX`: slot d2 expects GridX but found GridY"));
}
#[test] fn clean_shapes_draw_no_printed_family_refusal() // reaches "error: contract closure gate FAILED." (baseline rows), none of the 3 headers
#[test] fn sigil_contracts_0_still_skips_the_printed_families() // "warning: contract closure gate SKIPPED (SIGIL_CONTRACTS=0)." and none of the 3 headers
```

The fixture declares the two port names the bus analysis reads exactly as the frontend test spells them (declare them as `pub equ` at `$A11100` / `$C00004` if the build needs a definition).

- [ ] **Step 2: Run, verify FAIL.** `cargo test -p sigil-cli --test printed_families_gate` → first test FAILS on the bus header (today the build prints nothing for these families).
- [ ] **Step 3: Implement** `refuse_located` and three calls after the existing `empty_gate` rows, each setting `failed = true` when it returns true.
- [ ] **Step 4: Run, verify PASS** (all 3).
- [ ] **Step 5: RF.** Mutation (compiles, one line): `sed -i 's/if refuse_located("\[bus\.\*\]/if false \&\& refuse_located("[bus.*]/' crates/sigil-cli/src/main.rs`; quote the line with `grep -n 'if false && refuse_located' …`. Expect FAIL at the bus header assertion. Restore.
- [ ] **Step 6: Corpus.** BN → 7/7; AG → green (0 firings in all 7 shapes, MEASURED above, so no aeon change).
- [ ] **Step 7: Commit** `crates/sigil-cli/src/main.rs crates/sigil-cli/tests/printed_families_gate.rs docs/QUEUE.md` with the measured 0-firing counts in the message.

---

### Task 2 (A7): The clobber error says where

**Files:**
- Modify: `crates/sigil-frontend-emp/src/closure.rs:54-140` (`ProcNode` gains `decl_span: Option<Span>`), `:375-388` (`Firing` gains `span: Option<Span>`, `via: Option<String>`), `:514-560` (`check_firings` fills them)
- Modify: `crates/sigil-frontend-emp/src/corpus_contracts.rs:2360` and `:2446` (set `decl_span` from the proc / extern declaration)
- Modify: `crates/sigil-cli/src/main.rs:2244` (replace the `empty_gate` row with `refuse_located`)
- Create: `crates/sigil-cli/tests/clobber_gate_location.rs`

**Interfaces:**
- Consumes: `refuse_located` (Task 1).
- Produces: `closure::Firing { proc, reg, transitive, unbounded, span: Option<Span>, via: Option<String> }`; `fn first_carrier(node: &ProcNode, closure: &Closure, reg: &str) -> Option<String>` in `closure.rs` (the first callee in sorted `direct_callees` whose effective set holds `reg`, else `falls_into`, else `None`); `fn describe_clobber(f: &Firing, code: &str) -> String` in `main.rs` (Task 7 reuses it with `[call.clobbers-incomplete]`).
- Text (exact):
  - through a callee: ``{code} `{proc}` destroys `{reg}` through `{via}`, which its clobbers(...) does not declare``
  - direct: ``{code} `{proc}` destroys `{reg}` in its own body, which its clobbers(...) does not declare``
  - unbounded: ``{code} `{proc}` reaches an indirect call with no declared bound, so it may destroy every register; bound the call with `as Type` or declare `@allow("clobbers.unanalyzable", "<reason>")` ``

- [ ] **Step 1: Write the failing test.**

```rust
// FIXTURE (one 68k module):
//   proc Q () clobbers(d0-d1) { moveq #0, d0 / moveq #0, d1 / rts }
//   proc P () clobbers(d0)    { jbsr Q / rts }
//   proc R () clobbers(d0)    { moveq #0, d2 / rts }
//   proc U () clobbers(d0)    { jsr (a0) / rts }
#[test]
fn the_clobber_refusal_names_proc_register_path_and_line() {
    let err = build_stderr(tree(FIXTURE), false);
    assert!(err.contains("error: [proc.clobber-undeclared] closure firings, 3 firing(s); this family is zero-firing by contract:"));
    assert!(err.contains(&format!("  {}: [proc.clobber-undeclared] `P` destroys `d1` through `Q`, which its clobbers(...) does not declare", loc(&f, FIXTURE, "proc P"))));
    assert!(err.contains(&format!("  {}: [proc.clobber-undeclared] `R` destroys `d2` in its own body, which its clobbers(...) does not declare", loc(&f, FIXTURE, "proc R"))));
    assert!(err.contains(&format!("  {}: [proc.clobber-undeclared] `U` reaches an indirect call with no declared bound", loc(&f, FIXTURE, "proc U"))));
    assert!(!err.contains("closure firings, 3 firing(s); this family is zero-firing by contract.\n")); // the old location-free form
}
```

Plus a unit test in `closure.rs`: `first_carrier_names_the_callee_that_brings_the_register` (`P → Q`, reg `d1` → `Some("Q")`; reg `d0` written locally → `None`).

- [ ] **Step 2: Run, verify FAIL** (`cargo test -p sigil-cli --test clobber_gate_location`): the located lines are absent.
- [ ] **Step 3: Implement** the fields, `first_carrier`, `describe_clobber`, and the `refuse_located` row. `--report contracts` (`main.rs:2420-2430`) prints the same `via` text.
- [ ] **Step 4: Run, verify PASS**; run `cargo test -p sigil-frontend-emp --no-fail-fast` (Firing/ProcNode are shared; struct-literal test sites get the new fields).
- [ ] **Step 5: RF.** Mutation: restore the old row, `sed -i 's|refuse_located("\[proc.clobber-undeclared\] closure firings".*|empty_gate("[proc.clobber-undeclared] closure firings", report.firings.len());|'` on `main.rs` (one line; quote it). Expect FAIL on the `P` line. Restore.
- [ ] **Step 6: Corpus.** 0 closure firings in 7 shapes (MEASURED), so nothing new fires. BN 7/7; AG.
- [ ] **Step 7: Commit** the five paths.

Not in scope (booked in the parcel note): dropping the 63 per-file `[proc.clobber-undeclared]` warnings the closure finds clean (Sigil audit plan item 5, second half).

---

### Task 3 (A1): Range-check sized immediates, typed consts and bit numbers

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/code.rs` (new fns; all 12 `M68kBackend.lower_inst` call sites at `:340, :712, :733, :815, :928, :1016, :1135, :1147, :1260, :1267, :1309, :1727` go through the checked wrapper; `:1396-1405` message unified)
- Modify: `crates/sigil-frontend-emp/src/eval/const_arity.rs:97-120` (scalar range beside the arity check)
- Create: `crates/sigil-frontend-emp/tests/imm_range.rs`
- Modify: `docs/superpowers/notes/campaign-gap-ledger.md:4520` (book the `.emp` sized-immediate site into `CHECK-THEN-USE-RESIDUAL` as fixed)

**Interfaces:**
- Produces in `lower/code.rs`:
  - `fn lower_inst_checked(inst: &M68kInst, span: Span) -> Result<DataFragment, String>`: `check_imm_ranges` then `M68kBackend.lower_inst`.
  - `fn check_imm_ranges(inst: &M68kInst) -> Result<(), String>`: every `Operand::Imm(v)` of a sized instruction checked against `imm_bounds(size)`; bit-op source immediates checked by `bit_bound`.
  - `fn imm_bounds(size: Size) -> (i64, i64)`: `B → (-128, 255)`, `W → (-32768, 65535)`, `L → (i32::MIN as i64, u32::MAX as i64)` (the AS front end's `m68k_imm_bounds`, `frontend-as/src/eval.rs:14738-14744`).
  - `fn bit_bound(dest: &Operand) -> i64`: data register → 31, anything else → 7.
- Messages (the AS front end's, `frontend-as/src/eval.rs:3336`): `operand {v} out of range {lo}..={hi}`. The 32-bit message at `code.rs:1404` becomes the same form. Bit numbers (no AS equivalent; AS accepts them): `[lower.bit-number] bit number {n} out of range 0..={hi}`. Typed const (the data path's form): `[const.out-of-range] {v} does not fit {ty} ({lo}..={hi}), in '{name}'`.
- Untouched: `ImmLink` (link-time) immediates, and the encoder's own hard checks (moveq/addq/subq data, shift count 1..=8, trap 0..=15; MEASURED loud today).

- [ ] **Step 1: Write the failing tests** (helper `emp(src) -> Result<Vec<u8>, Vec<String>>` built on `parse_str` + `lower_module` + link, like `tests/tranche7_code_concat.rs:12-30`).

```rust
#[test] fn word_immediate_past_range_is_refused()  { assert_eq!(err("move.w  #$12345, d0"), ["operand 74565 out of range -32768..=65535"]); }
#[test] fn byte_immediate_past_range_is_refused()  {
    assert_eq!(err("move.b  #300, d1"),     ["operand 300 out of range -128..=255"]);
    assert_eq!(err("andi.b  #$1FF, d0"),    ["operand 511 out of range -128..=255"]);
    assert_eq!(err("cmpi.b  #300, d0"),     ["operand 300 out of range -128..=255"]);
    assert_eq!(err("move.b  #-129, d2"),    ["operand -129 out of range -128..=255"]);
    assert_eq!(err("andi.b  #~$80, d0"),    ["operand -129 out of range -128..=255"]);
    assert_eq!(err("andi.b  #$1FF, ccr"),   ["operand 511 out of range -128..=255"]);
}
#[test] fn sr_and_memory_destinations_are_checked() {
    assert_eq!(err("move.w  #$12345, sr"),  ["operand 74565 out of range -32768..=65535"]);
    assert_eq!(err("cmpi.w  #$12345, (a0)"),["operand 74565 out of range -32768..=65535"]);
    assert_eq!(err("addi.w  #-32769, d0"),  ["operand -32769 out of range -32768..=65535"]);
}
#[test] fn long_message_matches_the_as_front_end() { assert_eq!(err("move.l  #$123456789, d0"), ["operand 4886718345 out of range -2147483648..=4294967295"]); }
#[test] fn boundary_spellings_assemble() {
    assert_eq!(ok("move.b  #-128, d0"),  [0x10,0x3C,0x00,0x80]);
    assert_eq!(ok("move.b  #255, d0"),   [0x10,0x3C,0x00,0xFF]);
    assert_eq!(ok("move.w  #-32768, d0"),[0x30,0x3C,0x80,0x00]);
    assert_eq!(ok("move.w  #$FFFF, d0"), [0x30,0x3C,0xFF,0xFF]);
    assert_eq!(ok("andi.b  #~$7F, d0"),  [0x02,0x00,0x00,0x80]);
}
#[test] fn bit_numbers_are_checked() {
    assert_eq!(err("btst    #40, d0"),   ["[lower.bit-number] bit number 40 out of range 0..=31"]);
    assert_eq!(err("bset    #9, (a0)"),  ["[lower.bit-number] bit number 9 out of range 0..=7"]);
    assert_eq!(ok("btst    #31, d0"),    [0x08,0x00,0x00,0x1F]);
    assert_eq!(ok("bset    #7, (a0)"),   [0x08,0xD0,0x00,0x07]);
}
#[test] fn typed_const_out_of_range_is_refused() {
    assert_eq!(err_mod("const LIMIT: u8 = 300"), ["[const.out-of-range] 300 does not fit u8 (0..=255), in 'LIMIT'"]);
    assert_eq!(err_mod("const NEG: u8 = -1"),    ["[const.out-of-range] -1 does not fit u8 (0..=255), in 'NEG'"]);
    assert_eq!(err_mod("const SMALL: i8 = 200"), ["[const.out-of-range] 200 does not fit i8 (-128..=127), in 'SMALL'"]);
}
#[test] fn link_time_word_immediate_still_accepts_a_ram_address() {
    // a `vars` region label placed at $FFFF9EDE; `move.w #RamX, d0` → ImmLink path
    assert_eq!(ok_linked(RAM_FIXTURE), [0x30,0x3C,0x9E,0xDE, 0x4E,0x75]);
}
```

(`err` wraps one instruction in `module m / proc f() { … rts }` and returns the error messages without location; `ok` returns the bytes minus the trailing `4E75`.)

- [ ] **Step 2: Run, verify FAIL** (`cargo test -p sigil-frontend-emp --test imm_range`): every `err` case builds today (bytes in the Baseline table).
- [ ] **Step 3: Implement** the four fns and the const check; route all 12 call sites through `lower_inst_checked`.
- [ ] **Step 4: Run, verify PASS**; then `cargo test -p sigil-frontend-emp -p sigil-cli --no-fail-fast` (existing tests asserting the old 32-bit message are updated to the new text, nothing else).
- [ ] **Step 5: RF.** Mutation: insert `    return Ok(()); // RF-MUTATION` as the first body line of `check_imm_ranges` in `lower/code.rs`; quote it with `grep -n RF-MUTATION`. Expect FAIL in `byte_immediate_past_range_is_refused`. Restore.
- [ ] **Step 6: Corpus (the exact count).** Build all 7 shapes of the reference tree with the branch binary (BN does this). Expected 0 refusals (0 of 879 + 0 of 209 resolvable sites; the 584 sites grep cannot resolve are decided here). Any refusal is a line that ships truncated bytes today: STOP, list it in the parcel note, BLOCKED for the owner (fixing it changes bytes, aeon pair).
- [ ] **Step 7: BN 7/7, AG, commit** the four paths.

Not in scope (booked): `move.w $FFFF8000, d0` accepted as an immediate (EMP finding 3, third bullet; 0 corpus hits by grep).

---

### Task 4 (A8): CI green, and watched

**Files:**
- Modify: `crates/sigil-cli/tests/test_g4_final_objects_port.rs:277-288` (`compile_real_files`) and `:417-433` (`reference_gate`)
- Modify: `scripts/landing-run.sh` (a CI-shaped leg after the strict span)
- Modify: `crates/sigil-harness/tests/landing_verdict.rs` (one fixture test)
- Modify: `docs/OVERSEER.md` (the loop reads CI)

**Interfaces:**
- Produces: `const REAL_FILES: [&str; 7]` in the port test (`engine/system/types.emp`, `engine/objects/sst.emp`, `engine/system/constants.emp`, `engine/objects/objdef.emp`, `games/sonic4/config/constants.emp`, `games/sonic4/objects/test_player.emp`, `games/sonic4/objects/test_enemy.emp`), read by BOTH `compile_real_files` and the guard `let Some(_) = reference_tree(&REAL_FILES) else { return };` at the top of `reference_gate` (one list, so the guard cannot drift from the reads).
- Produces: `landing-run.sh` leg `CI-SHAPED`: `env -u AEON_DIR -u EMPYREAN_SUITE_ROOT SIGIL_ALLOW_PARTIAL=1 cargo test --workspace --no-fail-fast` into the same target dir; its rc `CI_RC` joins the `RESULT FAILED` condition beside `LEDGER_RC`; verdict line `  ci-shaped leg   <rc>`; `--verdict-only` reads it.

- [ ] **Step 1: Reproduce red.** `env -u AEON_DIR -u EMPYREAN_SUITE_ROOT SIGIL_ALLOW_PARTIAL=1 CARGO_TARGET_DIR=.target-ci cargo test -p sigil-cli --test test_g4_final_objects_port` → `1 failed` (`g4_objects_regions_match_reference`, `cannot read …/games/sonic4/objects/test_player.emp`), as CI run `36488380435`.
- [ ] **Step 2: Write the failing landing test** in `landing_verdict.rs`: `a_red_ci_shaped_leg_fails_the_landing` — a fixture log whose strict span is green and whose CI-SHAPED span ends `test result: FAILED`; assert exit 1 and a `RESULT          FAILED` line naming the CI-shaped leg. Run → FAIL (no such leg).
- [ ] **Step 3: Implement** `REAL_FILES` + guard, and the leg.
- [ ] **Step 4: Verify.** Step 1's command → `4 passed` with `skip:` lines; the landing test PASS; whole partial workspace `env -u AEON_DIR -u EMPYREAN_SUITE_ROOT SIGIL_ALLOW_PARTIAL=1 CARGO_TARGET_DIR=.target-ci cargo test --workspace --no-fail-fast` → 0 failed binaries; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] **Step 5: RF.** Mutation: delete the guard line from `reference_gate` (`sed -i '/let Some(_) = reference_tree(&REAL_FILES)/d'`; `git diff --stat` names the test file). Expect: Step 1 command FAILS again, and a `landing-run.sh` run shows `RESULT FAILED` from the CI-shaped leg. Restore.
- [ ] **Step 6: Watched.** Add to `docs/OVERSEER.md`'s loop: each loop runs `gh run list --branch master --limit 1 --json conclusion,headSha,createdAt`; a `failure` is the top row until fixed.
- [ ] **Step 7: CI proof.** Push `parcel/step0-soundness` (OWNER APPROVAL: pushing is owner-gated in this workspace) and `gh run watch` the branch run → `success`. After the merge, the first master run → `success`; record both run ids in the parcel note and A8's task note.
- [ ] **Step 8: Commit** the four paths.

---

### Task 5 (A2): Enforce region sizes, Z80 bank windows included

**Files:**
- Modify: `crates/sigil-ir/src/map.rs:98-170` (`validate_section`, `check_budget`)
- Modify: `crates/sigil-link/src/lib.rs:48-52` (`LinkedSection.region`), `:248` (fill it), `:947-956` (`window_refusal`), `:1043-1060` (`emit_rom` calls `check_regions`)
- Modify: `crates/sigil-harness/src/seam2.rs:531-532, 739, 964-965, 1047, 1204, 1378` (derived sizes; every seam-2 tail calls `check_regions`)
- Modify: `crates/sigil-harness/src/native.rs:4456-4481` (`check_budgets`), and its callers
- Modify: `crates/sigil-harness/src/map_placement.rs:21-115` (`when` on `[[region]]`; `map_for_shape`)
- Modify: `crates/sigil-harness/tests/seam2_region_unenforced.rs` (invert the four `gap_*` tests; file renamed `seam2_region_enforced.rs`)
- Create: `crates/sigil-cli/tests/region_enforcement.rs` (fixture: the `z80probe` tree, `audit-builds/revB-r2/z80probe/tf` + maps)
- Aeon pair: `games/sonic4/map.toml` `[[region]] z80_sound_bank` gains `when = "sound_on"` (and any other `kind = "z80_bank"` row under `games/*/map.toml`, found by grep)
- Modify: `docs/QUEUE.md:1631` (close `SEAM2-TABLE-REGION-UNENFORCED`)

**Interfaces:**
- `MemoryMap::validate_section(&self, name: &str, placed_in: Option<&str>, lma: u32, len: u32) -> Result<(), String>`. Rules, in order:
  1. `placed_in` names a map region (any kind) → the section must lie inside THAT region. (`place_sections` sets `group` to a region name, `resolve/mod.rs:1047`; native sets `declchain{n}`, `native.rs:3147`, which names no region and is ignored.)
  2. every `Z80Bank` region whose `[lma_base, +size)` contains `lma` must contain `lma + len`.
  3. today's rule: the first `Rom` region containing `lma` contains the end.
  `ObjectBank` regions are budgets, never containers (rule 1 aside).
- Refusal text (today's form): ``section `{name}` [{lma:#X},{end:#X}) overflows region `{region}` (ends {region_end:#X}), over by {n} bytes``.
- `pub fn check_regions(image: &LinkedImage, map: &MemoryMap) -> Result<(), String>` in `sigil-link` (the loop `emit_rom` runs today).
- `pub fn bank_room(lma: u32) -> u32 { 0x8000 - (lma & 0x7FFF) }` in `seam2.rs` (the `mt_bank` precedent at `:1563`), replacing the seven literals.
- `pub fn check_budgets(resolved: &[Section], map: &MemoryMap, pmap: &PlacementMap) -> Result<Vec<(String, u32)>, String>`: every `[[budget]]` row, cap = the row's `ceiling`; replaces `check_object_bank_budget` and the `"object_bank"` literal.
- `PlacementMap.region_when: BTreeMap<String, When>`; `pub fn map_for_shape(map: &MemoryMap, pmap: &PlacementMap, sound_on: bool) -> MemoryMap` drops regions whose `when` does not apply. Native passes the shape's map to `emit_rom`.

- [ ] **Step 1: Write the failing unit tests** in `map.rs`:

```rust
#[test] fn validate_placed_checks_the_named_region() {     // rom [0,0x8000) listed first, drv_pcm [0x300,0x304)
    assert_eq!(m.validate_section("drv_pcm", Some("drv_pcm"), 0x300, 7).unwrap_err(),
        "section `drv_pcm` [0x300,0x307) overflows region `drv_pcm` (ends 0x304), over by 3 bytes"); }
#[test] fn z80_bank_containment_refuses_a_section_crossing_the_window() { // rom [0,0x400000), z80_sound_bank [0xB8000,0xC0000)
    assert!(m.validate_section("sfx", None, 0xBDC5C, 0x3000).unwrap_err().ends_with("overflows region `z80_sound_bank` (ends 0xC0000), over by 3164 bytes")); }
#[test] fn z80_bank_exact_fill_is_accepted()            { assert!(m.validate_section("b1", None, 0xB8000, 0x8000).is_ok()); }
#[test] fn a_section_starting_at_the_window_end_is_outside_it() { assert!(m.validate_section("SongBank2_Head", None, 0xC0000, 0x100).is_ok()); }
#[test] fn object_bank_is_a_budget_not_a_container()    { assert!(m.validate_section("OJZ_Sec8_Blocks", None, 0x1FF86, 0x1660).is_ok()); }
#[test] fn every_budget_row_is_enforced()               // two [[budget]] rows; the second overflows by 1 → Err naming it
```

`seam2_region_enforced.rs`: the four `gap_*` tests become `enforced_*` asserting `Err` (single overflow, seam-2 tail overflow, spill starting at the next region's base, overflow of a region nested in an earlier `rom` region); add `z80_bank_two_sections_summing_past_the_window_are_refused` (reviewer B's); `measure_real_sound_tables_z80_through_seam2` asserts the derived room holds the measured 1,116 B.

`region_enforcement.rs` (CLI, `sigil emp --root . --map`): `nested_region_overflow_is_refused` (z3 map → exit 1, stderr has `over by 3 bytes`; today exit 0); `z80_window_region_with_unmet_when_is_not_enforced` (harness-level: `map_for_shape(.., sound_on=false)` drops a `when = "sound_on"` region).

- [ ] **Step 2: Run, verify FAIL**: the `z80_bank`, placed-in, nested and inverted tests fail; the accept tests pass already (they pin the boundary).
- [ ] **Step 3: Aeon pair first.** On `parcel/step0-soundness-pair`: add `when = "sound_on"` to the region row(s). Inert under the shipping binary (MEASURED: the key is accepted today). BN with base=cand=base binary over the pair tree → 7/7; commit in aeon.
- [ ] **Step 4: Implement** the interfaces above.
- [ ] **Step 5: Run, verify PASS**; `cargo test -p sigil-ir -p sigil-link -p sigil-harness -p sigil-cli --no-fail-fast`.
- [ ] **Step 6: RF (two gates).** (a) Mutation: delete rule 2's loop in `validate_section` (quote the removed `RegionKind::Z80Bank` line via `git diff`) → `z80_bank_containment_refuses_a_section_crossing_the_window` FAILS. Restore. (b) Mutation: pass `None` for `placed_in` in `check_regions` → `validate_placed_checks_…` passes but `nested_region_overflow_is_refused` FAILS (the CLI path). Restore.
- [ ] **Step 7: Whole-build red-first (aeon-shaped, reviewer B's e5+e7).** In a run-unique copy of the provisioned tree: add 23,008 B after the bank-1 SFX block and move anchor `song_bank_2` `0xC0000` → `0xC8000`. Base binary: exit 0 (audit: 883,143 B). Branch binary: exit 1 with ``overflows region `z80_sound_bank` (ends 0xC0000), over by 13888 bytes`` (bank 1 ran to `0xC3640`). Paste both.
- [ ] **Step 8: Corpus.** Bank 1 has 9,124 B headroom; the object-bank budget holds; derived seam-2 rooms hold every table → 0 refusals expected. BN 7/7; AG on the pair branch; SS (cross-seam). Land as a PAIR (aeon first).
- [ ] **Step 9: Commit** each path listed above, sigil and aeon separately.

---

### Task 6 (A9): Whole-ROM goldens at aeon HEAD

**Files:**
- Modify: `crates/sigil-harness/golden/{s4,s4.debug,demo,demo.debug,config_a,config_b,lean}.bin`, `golden/provenance.toml` (one appended entry), `golden/offcanonical_sizes/*.txt`, generated `pins.rs`, `repin.toml`
- Modify: port tests the post-freeze suite names (repaired by derivation, never by copying a regenerated pin)
- Create: `docs/superpowers/notes/2026-11-16-pin-advance-step0.md`

**Interfaces:**
- Consumes: Task 4 (CI trusted), Task 0 (provisioned tree at the target revision).
- Produces: provenance entry `name = "pin-advance-step0"`, `aeon_rev = <full sha>`, `ab = "sigil-side corpus pin advance 1ee78b88 to <rev8>, the quiet revision aeon named; s4 and s4.debug reproduce aeon's own figures"` (the `pin-advance-s2` form, `provenance.toml:11675-11678`).

- [ ] **Step 1: Name the revision.** Ask the aeon lane / hub for its quiet revision (the `pin-advance-s2` precedent). Today's candidate: `90a45adb` (64 commits past `1ee78b88`).
- [ ] **Step 2: Baseline** per `docs/superpowers/notes/2026-09-28-pin-advance-s2.md`: `refreeze --check` OK on the current tip; `repin --check` against the new tree (record rc and message); SS at unmodified branch tip against the new tree → record the failing-binary list as the work list.
- [ ] **Step 3: Repin and freeze.** Fix `repin.toml` rows by derivation (anchors and section ends, not literals) until `repin --check` resolves; `cargo run -p sigil-harness --bin refreeze -- --freeze pin-advance-step0 --ab "<the ab text>"`.
- [ ] **Step 4: Verify the freeze.** Recompute each golden's CRC/size with python `zlib.crc32`; s4 and s4.debug must equal aeon's own build of that revision (at `90a45adb`: s4 `193507d9`/850,296, sha256 `1d679068eb1180ff…`); demo and config_b equal to the previous entry (MEASURED unchanged at `90a45adb`). `refreeze --check` OK, chain length +1.
- [ ] **Step 5: Repair** each red port gate by deriving its new value, then comparing to the regenerated `pins.rs`. SS → `RESULT GREEN`.
- [ ] **Step 6: Commit** in the `pin-advance-s2` sequence (repin, freeze, repairs), each with its finding.

No new gate (the existing `provenance_chain_holds` and `refreeze --check` are the guards), so RF does not apply. Ongoing drift is not solved here: moving the golden to the aeon side is Sigil audit plan item 11, after the driver rewrite.

---

### Task 7 (A5): Z80 code under the clobber gate

**Files:**
- Create: `crates/sigil-frontend-emp/src/z80_closure.rs` (the closure now in `sigil-harness/src/seam1.rs:1037-1200`, generalised to the corpus)
- Modify: `crates/sigil-frontend-emp/src/corpus_contracts.rs:69,221` (`ContractReport` fields), the Z80 pass that feeds `compute_verified_z80_outs`
- Modify: `crates/sigil-cli/src/main.rs` (`run_contract_gate` row; `--report contracts` section)
- Modify: `crates/sigil-harness/src/seam1.rs:1017-1060` (thin wrappers; delete `is_opcode_dispatch_proc`)
- Modify: `crates/sigil-cli/tests/z80_clobbers_incomplete.rs` (partition by annotation, not by name)
- Create: `crates/sigil-cli/tests/z80_clobber_gate.rs`
- Aeon pair: `engine/sound/sound_sequencer.emp` — `@allow("clobbers.computed-dispatch", "reached only through SeqOpcodeTable's ex (sp),hl dispatch")` on the 26 `Seq_Op_*` procs and `Seq_ContinueFetch` (`:1825`)

**Interfaces:**
- Consumes: `refuse_located` (Task 1), `describe_clobber`, `Firing.span/via` (Task 2).
- Produces: `pub fn z80_clobber_firings(procs: &BTreeMap<String, ProcNode>) -> (Vec<Firing>, Vec<String>)` (firings, excluded proc names); `ContractReport.z80_clobber_firings: Vec<Firing>`, `ContractReport.z80_clobber_excluded: Vec<String>`; `ProcNode.computed_dispatch: bool` (set from the `@allow` rule `clobbers.computed-dispatch`).
- Gate header: `[call.clobbers-incomplete] (Z80) closure firings`; describe: `describe_clobber(f, "[call.clobbers-incomplete]")`. Report line: `-- [call.clobbers-incomplete] (Z80) closure firings ({n}), {m} excluded by @allow("clobbers.computed-dispatch") --`.

- [ ] **Step 1: Measure at HEAD.** `SIGIL_STRICT_GATE=1 AEON_DIR=<ref tree> cargo test -p sigil-cli --test z80_clobbers_incomplete --no-fail-fast` → record in-scope firings (expect 0, as `honest_corpus_no_in_scope_firings` at `1ee78b88`) and the dispatch-proc firing count, both shapes.
- [ ] **Step 2: Aeon pair first.** Add the 27 annotations (no code edit). BN base=cand=base over the pair tree → 7/7; commit in aeon.
- [ ] **Step 3: Write the failing test** (`z80_clobber_gate.rs`, fixture-tree pattern; reviewer B's e8 shape):

```rust
// Z80 module (cpu: z80):
//   proc Audit_Q () clobbers(a, b, f) { ld b, 0 / ld a, 0 / ret }
//   proc Audit_P () clobbers(a, f)    { call Audit_Q / ret }
#[test]
fn the_build_gate_refuses_a_z80_clobber_underclaim_with_its_location() {
    let err = build_stderr(tree(Z80), false);
    assert!(err.contains("error: [call.clobbers-incomplete] (Z80) closure firings, 1 firing(s); this family is zero-firing by contract:"));
    assert!(err.contains(&format!("  {}: [call.clobbers-incomplete] `Audit_P` destroys `b` through `Audit_Q`, which its clobbers(...) does not declare", loc(&f, Z80, "proc Audit_P"))));
}
#[test] fn a_computed_dispatch_allow_excludes_the_proc()  // same + @allow("clobbers.computed-dispatch", "fixture") on Audit_P → header absent
#[test] fn sigil_contracts_0_still_skips_the_z80_family()
```

- [ ] **Step 4: Run, verify FAIL** (today no Z80 line at any warning level; e8 MEASURED exit 0).
- [ ] **Step 5: Implement**; `z80_clobbers_incomplete.rs`'s partition test asserts `excluded` equals the 27 annotated names read from source (derived by scanning the tree for the attribute, not a typed list).
- [ ] **Step 6: Run, verify PASS**; `cargo test --workspace --no-fail-fast` under SS (cross-seam).
- [ ] **Step 7: RF.** Mutation: prefix the new `refuse_located("[call.clobbers-incomplete] (Z80)…` condition with `false && ` (Task 1's form; quote it) → first test FAILS. Restore. Whole-build: e8 on a run-unique copy of the pair tree (`Audit_P`/`Audit_Q` in `engine/sound/sound_psg.emp`): base exit 0, branch exit 1 with the located line.
- [ ] **Step 8: BN 7/7, AG on the pair branch; land as a PAIR (aeon first). Commit** each path.

---

### Task 8 (A4): One name environment per declaration

**Files:**
- Create: `crates/sigil-frontend-emp/src/resolve/hygiene.rs`
- Modify: `crates/sigil-frontend-emp/src/resolve/mod.rs:258-325` (`collect_pub_comptime` injects hygienic closures), `:381-466` (`ambient_items` / `ambient_from_uses` thread the memo)
- Modify: `crates/sigil-cli/src/main.rs` (`--report name-env`)
- Create: `crates/sigil-cli/tests/one_name_env.rs` (fixtures: the reviewer-A probes `p8`, `p8b`, `p2d`, `p2e` from `audit-builds/reviewer-a-r2/probes/`, plus the two Review Focus shapes)
- Aeon pair (doc, and code only if Step 2 finds unbound names): `docs/EMP_PITFALLS.md` §2 and §8 marked retired

**Interfaces:**
- `pub fn free_names(item: &ast::Item) -> BTreeSet<String>`: identifiers an item's types, initialisers and bodies reference, minus its parameters and locals.
- `pub fn hidden_name(def_id: &str, name: &str) -> String` → `format!("${def_id}${name}")` (the `mangled_equ` shape, `resolve/imports.rs:19-21`; unspellable from source, unique because a module has one item per name).
- `pub fn rename_free(item: &mut ast::Item, map: &BTreeMap<String, String>)`.
- `pub(crate) struct HygieneMemo` keyed by `(def_id, name)`; `pub(crate) fn hygienic_closure(def: &ParsedModule, name: &str, manifest: &Manifest, memo: &mut HygieneMemo) -> Vec<ast::Item>`: the item under its public name with each free name bound in the DEFINING module's environment (own items, private included, and its own imports, recursively in their own defining modules) renamed to `hidden_name`, plus every such dependency, each elaborated once per program. Prelude names, `-D` defines and `Game.*` contract members are global and not renamed.
- A free name bound nowhere at the definition is an error: ``[name.unbound-at-definition] `{name}` in `{item}` is not bound in module `{def}`; add a `use` there``.
- Consts keep today's definition-site fold (`resolve/mod.rs:299-318`); the hidden injection covers what the fold cannot (structs, enums, bitfields, newtypes, comptime fns, non-foldable consts).
- `sigil build --report name-env`: one row per (importing module, injected item, free name) with `bound-at-definition`, `caller-binding`, `differs` columns, then `-- name-env: {n} differ, {u} unbound-at-definition --`.

- [ ] **Step 1: Report first.** Implement `free_names` and `--report name-env` (no behaviour change). Unit tests: `free_names_excludes_params_and_locals`, `free_names_sees_struct_field_types`.
- [ ] **Step 2: Measure the corpus** over the reference tree, all 7 shapes. Expect `0 differ` (ESTIMATED 0 by the token probe). Record `u` (unbound-at-definition): the scene-registry comment (`scene_registry.emp:49-64`) shows helpers reaching names through caller imports, so `u > 0` is plausible. If `u > 0`, the aeon pair adds the missing `use` lines in the DEFINING modules (byte-neutral; BN proves it) and lands first. If `differs > 0`, STOP: those lines change meaning, owner call.
- [ ] **Step 3: Write the failing tests** (`one_name_env.rs`, `sigil emp main.emp --root .`):

```rust
#[test] fn struct_layout_is_computed_in_its_defining_module()        { assert_eq!(build(P8B), [0x70,0x06,0x4E,0x75]); }  // today 70 0C
#[test] fn struct_layout_needs_no_caller_binding()                    { assert_eq!(build(P8),  [0x70,0x06,0x4E,0x75]); }  // today an error at a.emp
#[test] fn comptime_fn_free_const_resolves_where_defined()            { assert_eq!(build(P2D), [0x70,0x05,0x72,0x05,0x4E,0x75]); } // today 09 09
#[test] fn comptime_fn_private_helper_resolves_where_defined()         { assert_eq!(build(P2E), [0x70,0x04,0x4E,0x75]); }  // today 06
#[test] fn struct_field_type_from_a_third_module_uses_that_modules_names() // a.S { t: b.T }, b.N = 2, main has N = 50 → sizeof(S) per b
#[test] fn private_helper_chain_resolves_where_defined()                // pub f → private g → private h; main defines g and h differently
#[test] fn an_importers_own_names_still_mean_themselves()               // main's `moveq #N, d0` with its own N = 10 → 70 0A
#[test] fn a_name_unbound_at_definition_is_refused()                    // helper reads LIMIT it never imports; caller has one → "[name.unbound-at-definition] `LIMIT` in `lim` is not bound in module `helper`; add a `use` there"
```

- [ ] **Step 4: Run, verify FAIL** (bytes as noted; MEASURED today).
- [ ] **Step 5: Implement** `hidden_name`, `rename_free`, `HygieneMemo`, `hygienic_closure`; call it from `collect_pub_comptime`.
- [ ] **Step 6: Run, verify PASS**; `cargo test -p sigil-frontend-emp -p sigil-cli -p sigil-harness --no-fail-fast`; then `--report name-env` again → `0 differ, 0 unbound-at-definition`.
- [ ] **Step 7: RF.** Mutation: make `hygienic_closure` return the bare clone (`sed` an early `return vec![item.clone()];`; quote it) → `struct_layout_is_computed_in_its_defining_module` FAILS with `70 0C`. Restore.
- [ ] **Step 8: Corpus proof.** BN 7/7 against the Task 6 goldens (every ensure and LinkAssert still evaluates; a changed verdict fails the build); AG; SS. Land as a PAIR if Step 2 added aeon lines. Update aeon `docs/EMP_PITFALLS.md` §2/§8 in the pair.
- [ ] **Step 9: Commit** each path.

Not in scope (booked): the contract gate's flat last-writer-wins name list (`corpus_contracts.rs:452-460`, Sigil finding 12) and the 12-module helper list (`native.rs:1802`); the 50 literal pins and the scene import tax become removable but are left for aeon to retire.

---

### Task 9 (A6): Front-end differential gate

**Files:**
- Create: `crates/sigil-cli/tests/support/frontend_diff.rs` (generator + both assemblers + comparison + named exclusions; the `support/capstone_diff.rs` sharing precedent)
- Create: `crates/sigil-cli/tests/frontend_differential.rs`
- Create: `crates/sigil-harness/tests/frontend_differential_coverage.rs` (single test: the capture tap is process-global, `m68k_capstone_stream.rs:28-33`)

**Interfaces:**
- Consumes: Task 3 (until it lands, this gate is red, which is its red-first proof).
- Produces in `support/frontend_diff.rs`:
  - `pub struct Case { pub family: &'static str, pub size: Option<char>, pub src: Ea, pub dst: Option<Ea> }`; `pub enum Ea { D(u8), A(u8), Ind(u8), PostInc(u8), PreDec(u8), Disp(i16, u8), AbsW(u16), AbsL(u32), Imm(i64) }`.
  - `pub fn cases() -> Vec<Case>`: deterministic, exhaustive over the families of `sigil_isa::m68k::ALL_FAMILY_NAMES` that take a source operand × sizes `b/w/l` × the `Ea` table; immediates take `{lo-1, lo, -1, 0, 1, hi, hi+1}` with `(lo, hi)` from Rust bounds (`i8::MIN as i64`, `u8::MAX as i64`, `i16::MIN`/`u16::MAX`, `i32::MIN`/`u32::MAX`). Z80: `ld r,n` / `ld rr,nn` / `cp n` with the `i8`/`u8`, `i16`/`u16` sets.
  - `pub fn emp_spelling(c: &Case) -> String`, `pub fn as_spelling(c: &Case) -> String`.
  - `pub fn emp_assemble(src: &str) -> Result<Vec<u8>, Vec<String>>` (`parse_str` + `lower_module` + link); `pub fn as_assemble(src: &str) -> Result<Vec<u8>, Vec<String>>` (`sigil_frontend_as::assemble` + link).
  - `pub enum Outcome { SameBytes, BothRefuse, Disagree { emp: Result<Vec<u8>, Vec<String>>, as_: Result<Vec<u8>, Vec<String>> } }`; `pub fn compare(c: &Case) -> Outcome`.
  - `pub const EXCLUSIONS: &[(&str, &str)]`: `(pattern, reason)`; the first row: bit numbers past 31/7, which `.emp` refuses (Task 3) and the AS route accepts, as asl does.
  - `pub fn case_triples() -> BTreeSet<(String, Option<char>, Vec<&'static str>)>` (family, size, EA class per operand).

- [ ] **Step 1: Write the tests.**

```rust
#[test] fn emp_and_as_front_ends_agree_on_every_generated_m68k_case() {
    let (n, bad) = run(cases_m68k());                      // bad = Disagree not matched by EXCLUSIONS
    assert!(bad.is_empty(), "{}", report(&bad));
    assert_eq!(n, expected_case_count());                  // the product of the generator's own tables, computed, not typed
}
#[test] fn every_out_of_range_immediate_is_refused_by_both() {
    let oor = cases_m68k().into_iter().filter(is_out_of_range).collect::<Vec<_>>();
    assert!(!oor.is_empty());
    assert!(oor.iter().all(|c| matches!(compare(c), Outcome::BothRefuse)));
}
#[test] fn emp_and_as_front_ends_agree_on_every_generated_z80_case()
#[test] fn every_exclusion_is_used()                       // an unused EXCLUSIONS row is a stale hole: fail
// frontend_differential_coverage.rs (reference-dependent: reference_tree(&["engine/system/vblank.emp"]), skip / strict panic):
#[test] fn generator_covers_every_family_size_ea_the_aeon_build_emits() {
    // CaptureSession::begin(); build s4 plain and s4 debug; drain; map each Instruction to a triple
    assert!(emitted.is_subset(&case_triples()), "{:?}", emitted.difference(&case_triples()));
}
```

- [ ] **Step 2: RF, natural form.** Run on the branch with Task 3's check disabled (Task 3's mutation, applied and quoted) → `emp_and_as_front_ends_agree…` FAILS listing `move.b #256,d1` and peers as `Disagree` (emp bytes, AS `operand 256 out of range -128..=255`). Restore → PASS. This is the gate that would have caught Sigil finding 2.
- [ ] **Step 3: Coverage.** `SIGIL_STRICT_GATE=1 AEON_DIR=<ref tree> cargo test -p sigil-harness --test frontend_differential_coverage` → PASS; if a triple is missing, extend the generator's tables (never the assertion).
- [ ] **Step 4: Second RF for coverage.** Mutation: drop `Ea::Disp` from the generator table → the coverage test FAILS naming `(d16,An)` triples. Restore.
- [ ] **Step 5: Wired.** Both files run in the default workspace test (CI runs the differential; the coverage test skips in CI and panics under strict). BN 7/7 (no source change). **Commit** the three paths.

---

## Landing sequence and BLOCKED/owner items

- Land in waves: {1, 2, 3, 4} → {5, 6} → {7, 8, 9}. Each landing: SS green, BN 7/7, AG green, CI green on the branch, then merge, rebuild the shared pair, re-run AG with the shared pair, and record `merge-base --is-ancestor` with the tip SHA (invariant 12).
- **Owner approval needed, not BLOCKED:** (1) this plan's direction (EMP audit decision 2, Sigil audit decision 1, both "Recommended: yes", status awaiting-owner); (2) pushing the branch to get a CI run (Task 4 Step 7); (3) the quiet aeon revision for Task 6 (aeon lane / hub names it).
- **Conditional stops, decided by measurement during execution:** Task 3 Step 6 finds a truncating line (byte-changing fix); Task 8 Step 2 finds a name whose meaning differs (`differs > 0`). Both are BLOCKED for the owner if they occur; both are expected 0.
- **Nothing is BLOCKED at planning time.**

## Self-review

- Spec coverage: A1 → Task 3; A2 → Task 5; A3 → Task 1; A4 → Task 8; A5 → Task 7; A6 → Task 9; A7 → Task 2; A8 → Task 4; A9 → Task 6. G0's "both wrong-ROM paths closed" = Tasks 3 + 5; "checks gate" = Tasks 1, 2, 7; "CI green" = Task 4.
- Every gate names its mutation (RF) and its runner (workspace `cargo test`, `landing-run.sh`, `sigil build`'s own gate).
- Names used across tasks: `refuse_located` (1 → 2, 7), `describe_clobber` and `Firing.span/via` (2 → 7), `step0_shapes_cmp.sh` (0 → all), `check_imm_ranges` (3 → 9's mutation).
- Review Focus lines each have a test: 1 and 2 in Task 3, 3 and 4 in Task 5, 5 in Task 8.
