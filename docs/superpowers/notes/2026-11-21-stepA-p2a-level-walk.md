# Step A piece 2a, the interrupt level walk inside one routine: working notes (baseline)

## Values (every later task uses these)

- Branch `parcel/stepA-p2a-level-walk`, worktree `/home/volence/sonic_hacks/.sigil-stepA-p2a`, cut from master `5a6c2dd875c2ecbc4a3d742b5d6aa4140622c7e7`.
- A11 `9f8e1bc6` is an ancestor of HEAD (`git merge-base --is-ancestor 9f8e1bc6 HEAD` exit 0).
- Scratch: `/home/volence/sonic_hacks/.stepA-p2a-scratch/`. BASE binary `.../sigil-base` (`sigil 0.1.0 (5a6c2dd8)`, tree clean), built with `CARGO_TARGET_DIR=.../target-base cargo build --release -p sigil-cli` (22s, incremental).
- Reference trees (reused, verified): aeon `/home/volence/sonic_hacks/.aeon-stepA-p1-ref` at `9b80db1d237b43d945fcef676a848e7007cfb839`, clean; Cherub `/home/volence/sonic_hacks/.cherub-stepA-p1-ref` at `8e478cb`. Cherub needs `AEON_DIR=<aeon ref>`.

## Baseline

- Shapes (`scripts/step0_shapes_cmp.sh --base B --cand B --aeon-ref AEON`, 40s): `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`. plain 142a39b9/850262, debug 20acdb67/870606, demo 4587df52/99980, demo_debug 37d66535/106981, config_a 2ce7438f/870972, config_b 128361b3/617715, lean f6b07847/798198. Same as piece 1.
- Contract reports (`build --project <p> --shape release --report contracts`, stdout, all rc 0), saved in scratch: `rn-base-sonic4.txt` 154 lines, `rn-base-demo.txt` 127, `rn-base-cherub.txt` 127 (same line counts as piece 1). Stderr in `*.err`.
- Front-end tests (`AEON_DIR=aeon ref`, `cargo test --release -p sigil-frontend-emp --no-fail-fast`, rc 0): 149 `test result:` lines, summed 2921 passed, 0 failed, 0 ignored, 0 measured. Log `tc.log`.
- Landing baseline: 6302 (`docs/QUEUE.md` line ~3169, STEP0-A11 entry: `reconciles 6233 baseline + 69 new = 6302 observed`, `RESULT GREEN` at `9f8e1bc6`).

## W2 "before" (all accepted by the base binary)

- `emp w2_planted.emp --hex`: rc 0, 18 bytes, no diagnostics (`... 54 8F 4E 75 ...`, the `addq.l #2,sp` exit).
- `emp a_leak_pop_d0.emp --hex`: rc 0, 18 bytes, no diagnostics.
- `emp e_bare_sr_planted.emp --hex`: rc 0, 18 bytes, one existing advisory `[proc.ccr-advisory]` (bare `preserves(sr)`, `rts` between save and restore); accepted.
- Whole-build plant: fresh `git clone` of the aeon ref at `9b80db1d` into `aeon-plant`. Line 183 of `engine/system/dma_queue.emp` before: `        move.w  (sp)+, sr` (the `.full:` exit); after: `        addq.l  #2, sp`. `build --project aeon-plant/games/sonic4 --shape release`: exit 0, `contract closure gate: passed (sonic4 release: 196 module(s) analysed ...)`, 850262 bytes, CRC ee52408a (differs from the clean 142a39b9, as the plant changes bytes). Restored with `git checkout 9b80db1d -- engine/system/dma_queue.emp`; `git status --short` empty; line 183 back to `move.w  (sp)+, sr`.
- Probe sources: `survey-probes/{w2_planted,a_leak_pop_d0,e_bare_sr_planted}.emp`; outputs `p-*.out`.

## Task 1: sp_effect (red-first)

Commits: 7f9a3f04 (feature + tests), c7cd74de (agreement test fix, below).

First RF pass on 7f9a3f04 found a test defect: with the immediate-cleanup arm
mutated to return `Push`, `sp_effect_reads_each_form` went red but
`sp_effect_agrees_with_the_stack_check` stayed green, because it compared `transfer`
against the table's expected value and never called `sp_effect`. Fixed in c7cd74de
(it now calls `sp_effect` and checks it against the table and `transfer`), then the
RF was redone on c7cd74de.

Mutation: the immediate-cleanup arm of `sp_effect` returns `Push` instead of `Drop`.
Mutated line from disk (`grep -n`):

    935:        return SpEffect::Push(n as u32);

`git diff --stat`: `crates/sigil-frontend-emp/src/preserves.rs | 2 +-`

`cargo test --release -p sigil-frontend-emp --lib sp_effect` RED:

    test preserves::frame_tests::sp_effect_agrees_with_the_stack_check ... FAILED
    test preserves::frame_tests::sp_effect_reads_each_form ... FAILED
    assertion `left == right` failed: addq.l #2,sp
      left: Push(2)
     right: Drop(2)
    test result: FAILED. 0 passed; 2 failed

Restored with `git checkout c7cd74de -- crates/sigil-frontend-emp/src/preserves.rs`;
`git status --short` clean for that path; re-run: 2 passed, 0 failed.

### Task 1 fix round (review: push-and-pop line)

Stated differences of `sp_effect` from `transfer`, now three: `pea <ea>` is `Push(4)`
(`transfer` does not model it); a `lea`/`pea` with an sp-based source is `Lost`
(`transfer` exempts `lea (sp), aN`); an instruction that is both push and pop
(`move.w (sp)+, -(sp)`) is `Lost` (`transfer` takes it as a push; real net is zero but
the slot is rewritten). Committed as 62dc8795.

RF A (mutate the guard to `if false && is_push(ops) && is_pop(ops) {`, line 956):
`sp_effect_reads_each_form` RED, `move.w (sp)+,-(sp)  left: Push(2) right: Lost`. The
agreement test stays green by design (that row is a stated difference, skipped).

RF B (a `.l` single push returns `Push(2)`, line 961): both tests RED. The agreement
test failed at `assert_eq!(moved, n as i64, "{name}")` (the `transfer`-side depth
comparison): `move.l d0,-(sp)  left: 4 right: 2`. Restored from 62dc8795, green.

## Task 2: sr_level state, meet, step (red-first)

Green committed first: 9866c3d5. Front-end 2951 passed, 0 failed (149 result lines; 2923 + 28).

RF A (restore arm returns the current level). Mutated line from disk:
`281:                                level,` (was `level: top.level,`); `git diff --stat`:
`sr_level.rs | 2 +-`. `save_then_restore_returns_the_saved_level` RED:
`left: SrState { level: Known(LevelSet(128)), .. depth: 0, saves: [] }`
`right: SrState { level: Known(LevelSet(256)), .. }` (level 7 instead of ENTRY).
Restored with `git checkout 9866c3d5 -- crates/sigil-frontend-emp/src/sr_level.rs`;
`git status --short` empty; green.

RF B (meet on different depths returns `self.clone()`). Inserted line from disk:
`141: (Stack::Known { depth: da, .. }, Stack::Known { depth: db, .. }) if da != db => return self.clone(),`;
`git diff --stat`: `sr_level.rs | 1 +`. `meet_of_different_depths_loses_the_stack` RED
(`left: .. stack: Known { depth: 0, saves: [] }` / `right: .. stack: Lost`).
`a_push_in_a_loop_settles` TERMINATED (finished in 0.00s) and was RED: the loop head kept
`Known { depth: 0 }` instead of `Lost`. Restored from 9866c3d5; `git status --short` empty;
all 28 sr_level tests green.

## Task 3: mask_findings walks a routine (red-first)

TDD RED: the 24 new tests were written first; `cargo test --release -p sigil-frontend-emp
--lib sr_level` failed to compile (17 errors: `mask_findings`, `MaskFindingKind`,
`BTreeSet` not found). GREEN: 52 sr_level tests passed (28 + 24). Green committed first:
1a59d26a. Front-end 2975 passed, 0 failed (149 result lines; 2951 + 24). Clippy
`--workspace --all-targets -D warnings` exit 0.

Found while building it:
- `Cfg::label_index(name)` is the first instruction at or after the label. A label followed
  by another label resolves to the instruction after both; a body-closing label (no
  instruction after it) is `None` and seeds nothing. Pinned in
  `a_closing_export_label_and_an_empty_body_do_not_panic`.
- The only single 68000 instruction in this tree that both writes `sr` and carries a
  leaving edge is the body's LAST instruction (its `FallOff` edge): no return or transfer
  mnemonic writes `sr`. `a_return_is_judged_after_its_own_effect` pins that both ways (a
  closing restore arrives at 7 and leaves clean; a closing `move.w #$2700, sr` arrives at
  the entry level and is found), the brief's `jmp Other` pair, and `rtr` (its Drop(2)
  loses the stack, not the level: clean).
- `sets_level(mnem, ops)` (`stop`, or `sr` as last operand) is the one test `step` and the
  unreached pass share; the pass adds `rte`. It does not also require
  `writes_dest_register` as `check_clobbers` does (dispatcher's resolution: same test as
  `step`).

Five red-first mutations, each applied to 1a59d26a, quoted from disk, run, restored with
`git checkout 1a59d26a -- crates/sigil-frontend-emp/src/sr_level.rs` (`git status
--short` empty after each) and re-run green. Every `git diff --stat`:
`crates/sigil-frontend-emp/src/sr_level.rs | 2 +-` (RF e: `| 3 ++-`).

RF a (drop the local-helper seeds). From disk:
`418:            CodeItem::Instr { mnemonic, ops, .. } if false && is_call(mnemonic) => {`
`a_local_helper_reached_only_by_bsr_is_walked` RED:
`left: [(3, Unreached)]` / `right: [(4, NotTheCallers { levels: [7] })]` (the helper is
never walked, so only the unreached pass sees its `move`).

RF b (drop the export-label seeds). From disk:
`417:            CodeItem::Label { name, export: true, .. } if false => at.extend(cfg.label_index(name)),`
`an_export_label_after_the_save_is_refused` RED: `left: []` /
`right: [(4, Untracked { lost_at: Some((3, "move", Span { start: 3, end: 4 })) })]`.

RF c (remove the `exit_diverges` skip). From disk:
`377:                    if false && exit_diverges(&items[idx], noreturn) {`
`a_diverging_rail_inside_a_masked_section_is_not_a_finding` RED:
`left: [(7, NotTheCallers { levels: [7] })]` / `right: []`.

RF d (remove the unreached pass). From disk:
`395:        if false && !states.contains_key(&idx) && (sets_level(mnemonic, ops) || mnemonic == "rte") {`
`a_helper_entered_through_a_pointer_is_unreached` RED: `left: []` / `right: [(4, Unreached)]`.

RF e (skip `FallOff`). From disk: `376:                Edge::FallOff => None,` (the arm split
off `Edge::TailOut | Edge::BranchOut`). `a_masked_branch_to_the_closing_label_is_found`
RED: `left: []` / `right: [(5, NotTheCallers { levels: [7] })]`.

All five: `test result: FAILED. 0 passed; 1 failed`, then after restore
`test result: ok. 1 passed; 0 failed`. Full transcript:
`.stepA-p2a-scratch/t3-rf.txt`. (The harness moved the RF run to the background after
180 s; it completed with exit 0 and the transcript was read only after all ten result
lines were present.)

### Task 3 Step 7: ask aeon before wiring

Temporary, uncommitted patch in `lower/proc.rs` `check_preserves_sr`, just before its
`sr_writes_round_trip` test (no diagnostic, no environment variable):

    for finding in crate::sr_level::mask_findings(&buf.items, noreturn) {
        eprintln!("P2A-FINDING {} {:?}", proc.name, finding);
    }

Built `cargo build --release -p sigil-cli` (target-cand), copied to
`.stepA-p2a-scratch/sigil-probe` (`sigil 0.1.0 (c731702c-dirty)`, 1 modified). Outputs and
stderr under `.stepA-p2a-scratch/step7/`. Every build rc 0, every `.bin` and `.err` present.

Probe is live: `sigil-probe emp step7/w2_planted.emp --hex` (rc 0) printed (twice, once
per call of `check_preserves_sr` for the proc):

    P2A-FINDING P MaskFinding { idx: 5, span: Span { source: SourceId(0), start: 145, end: 148 }, kind: NotTheCallers { levels: [7] } }

Shapes (`build --project <tree>/games/<game> --shape <row> -o step7/<game>-<row>.bin`,
`AEON_DIR=` the aeon ref; Cherub `build --project .cherub-stepA-p1-ref --shape release`):

| shape | rc | bytes / CRC (same as baseline) | `P2A-FINDING` lines |
|---|---|---|---|
| sonic4 release | 0 | 142a39b9/850262 | 0 |
| sonic4 debug | 0 | 20acdb67/870606 | 0 |
| sonic4 config-a | 0 | 2ce7438f/870972 | 0 |
| sonic4 config-b | 0 | 128361b3/617715 | 0 |
| sonic4 lean | 0 | f6b07847/798198 | 0 |
| demo release | 0 | 4587df52/99980 | 0 |
| demo debug | 0 | 37d66535/106981 | 0 |
| Cherub release | 0 | 800f166e/104533 | 0 |

No finding in any aeon or Cherub shape. Coverage: the probe sits in `check_preserves_sr`,
so it saw the procs that declare `preserves(sr)`/`preserves(sr.mask)` and reach the
round-trip test (32 such declarations in the aeon ref: the `Sound_*` API, `Music_Service`,
`QueueDMA_*`, `DMA_Deferrable_DestPending`, `VSync_Wait`, `Parallax_Update`,
`Section_RedrawPlanes`, `BG_Init`, `GameState_OJZScroll_Init`). Cherub declares none of its
own; its engine modules come from the aeon ref. Restored with
`git checkout c731702c -- crates/sigil-frontend-emp/src/lower/proc.rs`; `git status
--short` empty.

### Task 3 review fix: FallOff is always judged

Review finding (controller ruled the reviewer right, against the earlier resolution):
`exit_diverges` judges the whole instruction, not the edge, so skipping a `FallOff` on it
dropped ordinary exits: a passing assert's `AssertDesugar` `bcc .ok` to the body-closing
label, and the fall-through of a last `beq Fatal` with `Fatal` noreturn. Fixed in
4870e983: `FallOff` always judged on the OUT level; `TailOut`/`BranchOut` keep the skip
(as `ExitKind::Defer` in the stack check). New tests:
`a_masked_body_ending_in_a_passing_assert_is_found`,
`an_unmasked_body_ending_in_a_passing_assert_is_clean`,
`a_masked_last_branch_to_a_noreturn_target_still_judges_the_fall_through`,
`a_known_set_of_all_nine_bits_is_not_untracked` (`judge_level` on ENTRY + 0..7 →
`NotTheCallers [0..7]`). Comment in `a_known_level_with_the_entry_is_judged_by_its_fixed_levels`
corrected to {ENTRY, 3}. sr_level 56 passed; front-end 2979 passed, 0 failed (149 result
lines; 2975 + 4); clippy exit 0.

Minor 3 (seed helpers for any call operand form): NOT done. `flag_check::transfer_target_sym`
reads `Sym`, `SymOff`, `AbsSym`, not `PcRel`, so `jsr .h(pc)` returns `None`; per the
controller's rule seeding is left as it was (`Sym` only), and no
`a_helper_called_pc_relative_is_walked` test was added.

RF (put the skip back on `FallOff`). From disk:
`388:                Edge::FallOff => if exit_diverges(&items[idx], noreturn) { None } else { judge_level(out.level, &lost_at) },`
`git diff --stat`: `sr_level.rs | 2 +-`. `cargo test ... sr_level::tests::a_masked_` RED:
`a_masked_body_ending_in_a_passing_assert_is_found ... FAILED` (`left: []` /
`right: [(3, NotTheCallers { levels: [7] })]`) and
`a_masked_last_branch_to_a_noreturn_target_still_judges_the_fall_through ... FAILED`
(`left: []` / `right: [(2, NotTheCallers { levels: [7] })]`); `1 passed; 2 failed`.
Restored with `git checkout 4870e983 -- crates/sigil-frontend-emp/src/sr_level.rs`;
`git status --short` empty; sr_level 56 passed. Transcript `.stepA-p2a-scratch/t3fix-rf.txt`.

Step 7 re-run with the fixed code: same temporary probe in `check_preserves_sr`, binary
`sigil-probe` = `4870e983-dirty` (1 modified), outputs `.stepA-p2a-scratch/step7-fix/`.
W2 liveness (rc 0, printed twice):
`P2A-FINDING P MaskFinding { idx: 5, span: Span { source: SourceId(0), start: 145, end: 148 }, kind: NotTheCallers { levels: [7] } }`

| shape | rc | CRC/bytes (= baseline) | `P2A-FINDING` lines |
|---|---|---|---|
| sonic4 release | 0 | 142a39b9/850262 | 0 |
| sonic4 debug | 0 | 20acdb67/870606 | 0 |
| sonic4 config-a | 0 | 2ce7438f/870972 | 0 |
| sonic4 config-b | 0 | 128361b3/617715 | 0 |
| sonic4 lean | 0 | f6b07847/798198 | 0 |
| demo release | 0 | 4587df52/99980 | 0 |
| demo debug | 0 | 37d66535/106981 | 0 |
| Cherub release | 0 | 800f166e/104533 | 0 |

Every `.err` present. Reference trees' `git status --short` empty. Restored with
`git checkout 4870e983 -- crates/sigil-frontend-emp/src/lower/proc.rs`; `git status --short` empty.

### Task 4: the walk in place of the line-order test (7cef6d21)

`check_preserves_sr`'s third refusal (`if !sr_writes_round_trip(..)`) replaced by a loop over
`sr_level::mask_findings(&buf.items, noreturn)`: one `Level::Error` per finding at
`finding.span`, message texts as the brief; for `Untracked { lost_at: Some(..) }` also one
`Level::Note` at the `lost_at` span, "the interrupt level stops being known here"
(`sigil_span::Level` has `Note`). Two tail refusals and their returns untouched;
`sr_writes_round_trip` untouched (still called from `eval/asm.rs`). `dead_code` allowance on
`mod sr_level` removed; clippy `-D warnings` exit 0, no dead item reported.

**Why the check ran twice.** Temporary backtrace print in `check_preserves_sr` (removed with
`git checkout a213dac7 -- .../proc.rs`) on `survey-probes/d_ori_no_save.emp`: call 1 is
`lower_proc -> check_clobbers -> verified_preserves_regs -> check_preserves` into a local
`sink` (only "any error?" is read, to zero the register credit; the sink is dropped); call 2
is `lower_proc -> check_preserves` (line 258) into the real `diags`. Only call 2 reaches the
user. Measured: `sigil-base emp d_ori_no_save.emp --hex` prints the old diagnostic once;
`sigil-t4` prints the new one once (d_ori, w2_planted, e_bare_sr_planted: 1 line each;
h_via_d0: the error and the note, 1 each). Nothing done about it: the new diagnostics ride the
same path. Side effect (intended): a W2-shaped routine that also preserves registers now gets
no register credit from `verified_preserves_regs`, as any refused routine already did.

**TDD.** New `tests/sr_level.rs`, 18 tests, each pins the full diagnostic list (level, message
or code, anchor line via a byte-offset-to-line helper). RED before the wiring:
`7 passed; 11 failed` (W2 and variants, export-after-save, closing-label branch: `left: []`;
d/h/l/pointer: the old "not bracketed by" text at line 2; bare-`sr` W2: only the ccr advisory).
Log `.stepA-p2a-scratch/t4-red.log`. GREEN: 18 passed. Front-end total 2997 passed, 0 failed
(150 result lines; 2979 + 18). `tranche21_spelling_probes` 15 + `diag_assert_vector` 8 passed
(no skips), `sr_undeclared_gate` 6 passed. No existing test touched.

`with_ints_off_and_an_early_return_inside_is_refused`, measured: three errors, all on the inner
`rts` line: `[proc.preserves-sr-unbalanced]` (at 7), `[context.escape]`, `[stack.unbalanced]`
(2 bytes still on the stack). Before the wiring the first was absent.

**RF** (skip the findings loop). From disk:
`2627:    for finding in mask_findings(&buf.items, noreturn).into_iter().take(0) {`;
`git diff --stat`: `proc.rs | 2 +-`. `w2_is_refused_at_the_leaking_return ... FAILED`
(`left: []`, `right: [.. NotTheCallers text .., line: 8]`), `0 passed; 1 failed`. Restored with
`git checkout 7cef6d21 -- crates/sigil-frontend-emp/src/lower/proc.rs`; `git status --short`
empty; sr_level 18 passed. Transcript `.stepA-p2a-scratch/t4-rf.txt`.

**Sanity builds** (`sigil-t4` = `a213dac7-dirty`, the committed 7cef6d21 sources; outputs
`.stepA-p2a-scratch/t4-sanity/`): sonic4 release rc 0, 142a39b9/850262; sonic4 debug rc 0,
20acdb67/870606 (both = baseline); 0 `preserves-sr-unbalanced` lines. Aeon ref
`git status --short` empty.

## Where the answer changes from today

Copied from the plan, plus one row the Task 4 reviewer found (the last row).

| Shape | Today | After 2a |
|---|---|---|
| A path that masks and leaves without restoring (W2; also through `move.w (sp)+, d0`, under bare `sr`, or by `bra` to the closing label) | builds | refused |
| A write to `sr` before the save or after the last restore, when every path is still in order | refused | builds (correct) |
| A tail or conditional branch out of the routine while masked | builds unless it is the last instruction | refused |
| An `export` label after the save | builds for the routine itself (only entrants are refused) | refused |
| A restore after the stack stopped being followable earlier in the routine (`subq #n, sp`, `link`, a `d(sp)` store) | builds | refused. No aeon routine that makes the promise does this today (0 of 32) |
| A `targets(...)` dispatch inside a masked section, every arm restoring | builds | refused: the `jmp` is judged as a way out at the masked level, and since the fix wave each arm is walked as an extra entry and its restore is untracked. A known limit; none in aeon |
| A call to a routine that masks and does not restore | builds | builds (piece 2b) |
| Several findings in one routine | one diagnostic, on the `proc` line | one per way out, on that instruction |
| An `sr` write that leaves the mask bits alone with no save at all (`andi.w #$fffe, sr`; `ori.w #$0001, sr`) | refused | builds (correct for `sr.mask`: the mask is unchanged). Caveat for BARE `preserves(sr)`: a write to the S or T bits (`eori.w #$2000, sr`, `ori.w #$8000, sr`) also only gets `[proc.ccr-advisory]` now, because bare `sr` is mask + ccr and S/T are in neither half (was an error on master). Owner decision: `docs/QUEUE.md` STEPA-P2A-OWNER-NOTES |

## Task 5: whole-tree proof

Candidate: committed tree `7d46ec3d`, `git status --short` empty before the build,
`CARGO_TARGET_DIR=.stepA-p2a-scratch/target-cand cargo build --release -p sigil-cli`, copied to
`.stepA-p2a-scratch/sigil-cand-7d46ec3d`; `--version`: `sigil 0.1.0 (7d46ec3d)`, `tree: clean at
capture, no uncommitted changes` (no `-dirty`).

**BN** (`scripts/step0_shapes_cmp.sh --base sigil-base --cand sigil-cand-7d46ec3d --aeon-ref <aeon ref>`, rc 0):
```
SHAPE plain 142a39b9/850262 142a39b9/850262 SAME
SHAPE debug 20acdb67/870606 20acdb67/870606 SAME
SHAPE demo 4587df52/99980 4587df52/99980 SAME
SHAPE demo_debug 37d66535/106981 37d66535/106981 SAME
SHAPE config_a 2ce7438f/870972 2ce7438f/870972 SAME
SHAPE config_b 128361b3/617715 128361b3/617715 SAME
SHAPE lean f6b07847/798198 f6b07847/798198 SAME
SHAPES 7/7 IDENTICAL
STEP0_CMP_END rc=0
```

**RN** (`build --project <p> --shape release --report contracts`, candidate, all rc 0; `diff` against the Task 0
files): sonic4 diff=0, 154 lines; demo diff=0, 127 lines; Cherub diff=0, 127 lines. Three SAME. Files
`rn-cand-*.txt` in scratch.

**W2 whole-build plant** (fresh clone `aeon-plant` at `9b80db1d`, clean):
- Before, `sed -n 183p engine/system/dma_queue.emp`: `        move.w  (sp)+, sr`
- After the plant: `        addq.l  #2, sp`
- Candidate, `build --project games/sonic4 --shape release`: rc 1:
  `.../aeon-plant/engine/system/dma_queue.emp:185:9: [Error] [proc.preserves-sr-unbalanced] \`QueueDMA_Deferrable\` declares \`preserves(sr.mask)\` but this way out can be reached with the interrupt level at 7, not the caller's (a path masks interrupts and does not restore the saved level)`
  (line 185 is the `rts` after the planted line 183; line 184 is the `ori.b #1, ccr`).
- BASE binary, same planted tree: rc 0, 850262 bytes (the before/after).
- `git checkout 9b80db1d -- engine/system/dma_queue.emp`; `git status --short` empty; line 183 restored.
  Candidate rebuild: rc 0, 850262 bytes, CRC `142a39b9`.

**Aeon's own `build.sh`, four shapes: BLOCKED.** `SIGIL_BUILD` = candidate `sigil`, `SIGIL_EMIT` =
`emit_sound_blob` from `cargo build --release --locked -p sigil-cli -p sigil-harness --bins` (same target dir),
run in `aeon-plant` (restored, clean). `build.sh` printed `Assembler: sigil 7d46ec3dafbe (clean at capture ...)`
and then every one of the four (`./build.sh`, `DEBUG=1 ./build.sh`, `./build.sh demo`, `DEBUG=1 ./build.sh demo`)
exit 1 in its pre-build pytest lane, before any sigil build: `1 failed, 4262 passed, 3 skipped, 43 deselected`
(263 s, 251 s, 338 s, 244 s). The one failure each time, unrelated to Sigil:
`FAILED tools/test_suite_paths.py::test_the_walk_records_its_step_in_the_layout_this_run_actually_has`:
`AssertionError: UNMEASURABLE: the walk answered /home/volence/sonic_hacks, putting this resolver at
.stepA-p2a-scratch/aeon-plant/tools/suite_paths.py, which matches none of the layouts this repo has
({'main-checkout': ..., 'nested-worktree': ('<repo>', '.claude', 'worktrees', '<wt>', 'tools', 'suite_paths.py')})`.
The test refuses a checkout at a path of that shape. No workaround attempted (brief). The debug shapes are
covered at the sigil level by BN (debug 20acdb67/870606, demo_debug 37d66535/106981 identical to base), and
no `preserves-sr-unbalanced` appeared in any of the four logs. Logs `b-*.log` in scratch.

**TC** (`AEON_DIR=<aeon ref> ORACLE_DIR=/home/volence/sonic_hacks/oracle-old`, `cargo test --release
--workspace --no-fail-fast`, rc 0, log `tc5.log`): 569 `test result:` lines, 6378 passed, 0 failed, 4 ignored.
Expected: 6302 + 76 (`git diff 5a6c2dd8..HEAD -- crates | grep -cE '^\+[[:space:]]*#\[test\]'` = 76, `^-` = 0)
= 6378. Observed 6378, difference 0. `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
(Not the strict landing run.)

`docs/QUEUE.md`: added `STACK-PEA-UNMODELLED`, `STEPA-P2B-CALLS`, `STEPA-P2A-FALSE-REFUSALS`.

## Final review fix wave

Review: `.stepA-p2a-scratch/final-review/review.md` (verdict "With fixes": I1, I3 to fix; I2 to queue; minors).
Commits, each with its finding in the body:

| commit | fix |
|---|---|
| a3904942 | F3 (M6): `Untracked` has no `lost_at`; `loss_point`, the `Level::Note` and "stops being known at an earlier `{mnemonic}`" removed. Message: "... but the interrupt level cannot be followed to this way out (restore it with `move.w (sp)+, sr` from the matching `move.w sr, -(sp)`, with the stack as it was at the save)". `the_loss_point_skips_assert_lines` deleted (it tested only `loss_point`). |
| 4c5bd15b | F1 (I1): every instruction is walked. While an instruction is unreached, the lowest-index one is added as one more seed at `SrState::at_entry()` and the walk reruns with all seeds. `Unreached` kind, pass and message removed. |
| c6a49ddc | F2 (I3): `sp_effect` (not `transfer`) returns `Lost` for (a) a read exemption with a second sp operand, (b) any bare-`a7` destination other than the immediate cleanup (own guard), (c) a `movem` list with a7, (d) two or more `-(sp)`/`(sp)+` operands (subsumes the push-and-pop rule). 14 table rows; 9 are stated differences from `transfer` (measured by a temporary print of `transfer` per row, removed before commit); the doc now lists seven stated differences. |
| 2410158b | F4 (M7): the four stale comments in `lower/proc.rs` (dispatch ~1831, bare-`sr` tail note ~2578, `sr.ccr` slice ~2745, `sr_writes_round_trip` ~2988) now say the mask half is judged by `sr_level::mask_findings`. Comments only. |
| 5c01239d | F5 (M10): whole-source tests for the rows that now build (write before the save, write after the last restore, `andi.w #$fffe, sr` with no save: each measured rc 1 on `sigil-base`, clean now) and `eori.w #$2000, sr` under bare `sr` pinned to only `[proc.ccr-advisory]`. |
| (this commit) | F6: `docs/QUEUE.md` (`CFG-BT-BF-EDGES` new, `STACK-PEA-UNMODELLED` / `STEPA-P2B-CALLS` / `STEPA-P2A-FALSE-REFUSALS` extended and re-measured, `STEPA-P2A-OWNER-NOTES` new) and these notes. |

**TDD.** RED before each code fix, logs in scratch:
- F1 (`ff-f1-red.log`): 7 unit tests and 2 whole-source tests failed on 4c5bd15b's parent with the new tests in place, e.g.
  `a_pointer_entered_block_feeding_the_restore_is_found` `left: []` / `right: [(6, Untracked)]`,
  `a_targets_entered_block_feeding_the_restore_is_found` the same, `a_balanced_helper_entered_only_through_a_pointer_is_clean`
  `left: [(5, Unreached), (6, Unreached)]` / `right: []`, `a_dead_block_feeding_the_restore_is_refused` `left: []`.
  GREEN (`ff-f1-green.log`): sr_level unit 60, whole-source 19.
- F2 (`ff-f2-red.log`): `copying_the_top_over_the_saved_word_is_found` `left: []` / `right: [(5, Untracked)]`,
  `a_double_pop_on_one_line_is_found` `left: []` / `right: [(7, Untracked)]`, `sp_effect_reads_each_form`
  `move.w (sp),2(sp)  left: None  right: Lost`, and both whole-source twins `left: []`. GREEN (`ff-f2-green.log`):
  sr_level 62, sp_effect 2, whole-source 21.

**RF** (after committing green; transcripts `ff-f1-rf.txt`, `ff-f2-rf.txt`):
- F1 on 4c5bd15b. From disk: `368:    while let Some(idx) = first_unreached(items, &states).filter(|_| false) {`;
  `git diff --stat`: `sr_level.rs | 2 +-`. `a_pointer_entered_block_feeding_the_restore_is_found ... FAILED`,
  `left: []` / `right: [(6, Untracked)]`, `0 passed; 1 failed`. Restored with `git checkout 4c5bd15b -- <path>`;
  `git status --short` empty; `1 passed`.
- F2 A on c6a49ddc (movem-a7 guard off). From disk:
  `975:    if false && reglist_mask(ops).is_some_and(|m| m & (1 << 15) != 0) {`; `preserves.rs | 2 +-`.
  `sp_effect_reads_each_form ... FAILED`: `movem.l (sp)+,a7  left: Drop(4)  right: Lost`. Restored, status empty.
- F2 B on c6a49ddc (double sp-auto guard off). From disk:
  `971:    if false && ops.iter().filter(|o| is_sp_auto(o)).count() >= 2 {`; `preserves.rs | 2 +-`.
  `a_double_pop_on_one_line_is_found` (`left: []` / `right: [(7, Untracked)]`) and `sp_effect_reads_each_form`
  (`move.w (sp)+,-(sp)  left: Push(2)  right: Lost`, the first row the guard covers) FAILED, `0 passed; 2 failed`.
  Restored from c6a49ddc, status empty, `3 passed`.

**F1 STOP item: a case where the ruling's reasoning does not hold.** The reasoning assumes the extra seed is where
an unreached block is entered. A pointer or `targets` jump can land in the MIDDLE of an unreached region, after
code that never runs. The walk then reaches the real entry from the dead prefix, with the dead prefix's save on
the stack, and never seeds it at the caller's level. Measured (`ff-probes/`):

```
module m
proc P () clobbers(d0, d1, a0) preserves(sr.mask) {
    lea .mid(pc), a0
    jmp (a0)
.top:
    move.w sr, -(sp)
    move.w #$2700, sr
.mid:
    move.w (sp)+, sr
    rts
}
```

| probe | sigil-base | sigil-cand-7d46ec3d (branch before the wave) | sigil-cand-5c01239d (wave) |
|---|---|---|---|
| x1 (above) | rc 0 | rc 1 (2 x `Unreached`) | rc 0 |
| x2 (`.top` only saves) | rc 0 | rc 1 (`Unreached` on the restore) | rc 0 |
| x3 (same through `jmp .tbl(pc, d1.w) targets(.mid)`) | rc 0 | rc 1 | rc 0 |
| x4 (pointer into a REACHED restore: `beq .x` ... `.x: lea .mid(pc), a0 / jmp (a0)`) | rc 0 | rc 0 | rc 0 |

On the machine, `jmp (a0)` reaches `.mid` at depth 0 and `move.w (sp)+, sr` loads the high word of the return
address. x1 to x3 are refused before the wave and build after it: F1 as ruled closes c01/c02 and opens these.
None is worse than master. x4 is the same root (a pointer target is an entry the walk does not seed) in
reached code, open on all three binaries. Not adjusted, per the brief; reported for a ruling. A direction that
covers all four: seed at `at_entry()` every label whose address the routine takes (a `lea`/`pea`/`move` pc-relative
or absolute operand, a `targets(...)` list), in addition to the unreached loop; it needs its own aeon measurement
(a label on a data table would seed the instruction after the table).

**Probes re-run** (every file in `final-review/p/`, 72; table `ff-probe-table.txt`; before = `sigil-cand-7d46ec3d`,
after = `sigil-cand-5c01239d`). Closed (built clean before, refused now): `c01`, `c02` (I1), `s02`, `d12`,
`d15`, `c03` (I3). `c04`, `c05` were already refused by `[stack.unbalanced]` and now also get the level error.
Still building clean, by design: `q04`, `m_bt`, `m2_bt_s`, `m2_bt_w` (`bt`) and `q03`, `m2_bf` (`bf`), I2, recorded
as `CFG-BT-BF-EDGES`, not fixed; `h01`, `q08` (helper rewrites the caller's save) and `t01`, `t02` (tail or branch
out to a non-preserver), in `STEPA-P2B-CALLS`; `d10` (raw `dc.w`), in `STEPA-P2A-OWNER-NOTES`; `b01`..`b05` (bare
`sr` S/T/CCR bits, advisory only), owner note. Correct builds: `b04`, `d14`, `f04`, `q06`, `q07`, `m_trapv`,
`m_chk`. Every other probe: same verdict before and after.

**Whole tree** on `sigil-cand-5c01239d` (built from the committed tree, `git status --short` empty; `--version`
`sigil 0.1.0 (5c01239d)`, `tree: clean at capture`):
- BN (`env -u AEON_DIR scripts/step0_shapes_cmp.sh --base sigil-base --cand sigil-cand-5c01239d --aeon-ref <aeon ref>`,
  rc 0; with `AEON_DIR` set the script stops on `[engine.conflict]`, rc 2): all seven SAME, `SHAPES 7/7 IDENTICAL`,
  `STEP0_CMP_END rc=0`. So no aeon shape fires `[proc.preserves-sr-unbalanced]`.
- RN (`build --project <p> --shape release --report contracts`, all rc 0): sonic4 154 lines, demo 127, Cherub 127,
  each identical to the Task 0 `rn-base-*.txt` (`ff-rn-*.txt`). Cherub builds. Reference trees' status empty.
- W2 plant (`aeon-plant` at `9b80db1d`, line 183 `move.w (sp)+, sr` -> `addq.l #2, sp`): candidate rc 1,
  `dma_queue.emp:185:9: [Error] [proc.preserves-sr-unbalanced] \`QueueDMA_Deferrable\` declares
  \`preserves(sr.mask)\` but this way out can be reached with the interrupt level at 7, ...`; base rc 0, 850262 bytes.
  Restored with `git checkout 9b80db1d -- engine/system/dma_queue.emp`, status empty; candidate rebuild rc 0,
  850262 bytes, CRC `142a39b9`.
- TC (`AEON_DIR=<aeon ref> ORACLE_DIR=/home/volence/sonic_hacks/oracle-old`, split in three because one run did not fit
  the 10-minute limit: `--workspace --exclude sigil-harness --exclude sigil-frontend-emp` 349 lines 2849 passed,
  `-p sigil-frontend-emp` 150 lines 3010, `-p sigil-harness` 70 lines 532; all rc 0): 569 `test result:` lines,
  6391 passed, 0 failed, 4 ignored. Expected 6302 + 89 (`^+ #[test]` in `5a6c2dd8..HEAD -- crates`) - 0 = 6391.
  Difference 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.

### F1b: which labels are extra entries (1cd657b0)

Coordinator ruling after the F1 STOP. After a first walk from the ordinary entries, `mask_findings` also seeds
at `SrState::at_entry()`: (a) every code label whose instruction that walk did not reach; (b) every label in any
`targets` list; (c) every code label whose address the routine takes (a symbol in an operand of an instruction
that is not a branch, call or return: `is_transfer`). The lowest-unreached loop stays; seeds are a `BTreeSet`.
Symbols are read by `operand_symbols`: `Sym`, `SymOff`, `PcRel`, `PcRelIdx`, `AbsSym`, `DispSymInd`, and the
`Expr::Sym` leaves of `ImmLink` and `Z80MemSym`; no other variant carries a symbol. `Cfg::label_index` returns the
first instruction at or after a label EVEN WITH DATA BETWEEN, so `code_label_index` also requires that only
labels and context marks sit between the label and that instruction (a data label is not an entry).

TDD: 10 new unit tests and 5 whole-source tests; RED on 5c01239d (`ff-f1b-red.log`): 7 unit and 4 whole-source
failed (`left: []` for x1, x2, x3, x4, the unreached-label and reached-targets tests; the masked-dispatch test
missing its `(13, Untracked)`); the two data-label tests and the balanced-mid-block test were already green
(they pin non-entries). GREEN (`ff-f1b-green.log`): sr_level unit 71, whole-source 30; front-end 3024 passed.

RF on 1cd657b0 (`ff-f1b-rf.txt`), each quoted from disk, `git diff --stat` `sr_level.rs | 2 +-`, restored with
`git checkout 1cd657b0 -- <path>`, status empty, re-run green:
- (a) `469:                    if false && !first.contains_key(&idx) {` -> `a_label_in_unreached_code_is_an_entry`
  FAILED, `left: []` / `right: [(5, Untracked)]`.
- (b) `475:                for t in targets.iter().filter(|_| false) {` -> `a_reached_targets_label_is_an_entry`
  FAILED, `left: []` / `right: [(9, Untracked)]`.
- (c) `478:                if false && !is_transfer(mnemonic) {` -> `a_pointer_into_a_reached_restore_is_found`
  FAILED, `left: []` / `right: [(6, Untracked)]`.
- data label `501:    (between_is_code || true).then_some(target)` ->
  `a_data_label_whose_address_is_taken_is_not_an_entry` FAILED, `left: [(8, Untracked)]` / `right: []`.

**F1b STOP item: an entry need not land on a label.** Address arithmetic reaches an instruction with no label,
and the walk seeds the label instead. Each was refused on 7d46ec3d (the old unreached pass saw the sr writes) and
builds on 5c01239d and 1cd657b0; master builds all three. On the machine each lands on the restore at depth 0 and
loads the high word of the return address into `sr`.

```
module m
proc P () clobbers(d0, d1, a0) preserves(sr.mask) {
    lea .x(pc), a0
    jmp 6(a0)              ; y1. y2: `lea .x+6(pc), a0 / jmp (a0)`. y3: `jmp .x+6(pc)` on one path
.x:
    move.w sr, -(sp)
    move.w #$2700, sr
    move.w (sp)+, sr       ; .x+6
    rts
}
```

y1 bytes on 1cd657b0: `41 FA 00 06 4E E8 00 06 40 E7 46 FC 27 00 46 DF 4E 75` (`.x` = 8, `6(a0)` = 14 = `46 DF`).
Not adjusted. Seeding by label cannot see these; options for the ruling: treat every instruction as an entry when
the routine has a computed transfer without `targets` or takes a label's address with an addend (sound, refuses
every bracket in such a routine), or refuse `preserves(sr.mask)` on such routines outright.

Re-measure on `sigil-cand-1cd657b0` (committed tree, `tree: clean at capture`):
- Probes (72 review probes + x1-x4, y1-y3, r1-r5; table `ff-probe-table-f1b.txt`, columns 7d46ec3d / 5c01239d /
  1cd657b0). Differences from 5c01239d: x1, x2, x3 refused again, x4 refused for the first time, r5 newly refused
  (false refusal, QUEUE). y1-y3 build (STOP item). Every other probe as on 5c01239d.
- BN `SHAPES 7/7 IDENTICAL` (`AEON_DIR` unset for the script); RN sonic4 154 / demo 127 / Cherub 127, all SAME;
  Cherub builds; no `preserves-sr` line in any stderr. No aeon or Cherub shape is refused.
- W2 plant: rc 1 at `dma_queue.emp:185:9` (`QueueDMA_Deferrable`, level 7); restored, status empty, rebuild rc 0,
  850262 bytes, CRC `142a39b9`.
- TC, five runs (the 10-minute limit; a first `-p sigil-cli` run under `timeout 590` was cut short at 105 lines,
  rc 124, and was discarded and re-run in full): non-emp leaf crates 140 lines 1640, `sigil-cli` 203 lines 1029,
  `sigil-build` 6 lines 180, `sigil-harness` 70 lines 532, `sigil-frontend-emp` 150 lines 3024; all rc 0. 569
  lines, 6405 passed, 0 failed, 4 ignored. Expected 6302 + 103 - 0 = 6405. Difference 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
