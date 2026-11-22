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
