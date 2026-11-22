# Step A piece 2a: the interrupt level walk inside one routine. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A routine that declares `preserves(sr.mask)` or `preserves(sr)` is refused unless every path through its own body leaves at the caller's interrupt level. This closes the language review's W2.

**Architecture:** A new module `sr_level.rs` carries "which interrupt levels are possible here, and which saved levels are on the stack" along every path, using the shared walker `context::forward_in_states`. It replaces the one line-order test inside `check_preserves_sr` (`lower/proc.rs`). It is a per-file `Level::Error`, exactly where today's refusal lives, so `sigil emp`, `--check` and `sigil build` all get it. Calls keep today's reading (a call does not change the level); piece 2b replaces that with whole-program summaries.

**Tech Stack:** Rust, crate `sigil-frontend-emp`. No new dependency.

**Spec:** `empyrean/docs/v2/specs/2026-11-21-step-A-machine-state-checker-design.md` (sections "Piece 2" and "Piece 1 is built: what it taught piece 2"). Facts survey this plan is drawn from: `/home/volence/sonic_hacks/.stepA-p2-scratch/piece2-survey.md` (sigil `9f8e1bc6`, aeon `9b80db1d`); every `file:line` below is at that sigil commit.

## Scope: what 2a is, and what waits for 2b

| In 2a | Waits for 2b (own plan, written after 2a lands) |
|---|---|
| Path-following proof for the routine's own instructions | What a called routine does to the level (today: ignored; 22 callees of the declaring routines declare nothing) |
| `move`/`ori`/`andi`/`eori` to `sr`, `stop`, save and restore through the stack | The rule "inside `with ints_off { }` the level the acquire set holds until the release" |
| Every return, tail and branch out of the routine is judged | Whether a tail target preserves the level, beyond today's same-file rule |
| | Handlers entered at a known level (needs piece 3's table) |

## Global Constraints

- **Start only after A11 is on sigil master.** Branch `parcel/stepA-p2a-level-walk` from master, in worktree `/home/volence/sonic_hacks/.sigil-stepA-p2a`. Never commit to master. Never enter `/home/volence/sonic_hacks/.sigil-a11` or any other session's worktree.
- **No ROM byte changes.** This piece changes only what Sigil refuses. `scripts/step0_shapes_cmp.sh` must print `SHAPES 7/7 IDENTICAL` against the Task 0 baseline.
- **The contract report for sonic4, demo and Cherub is identical** before and after (`--report contracts`; Cherub needs `AEON_DIR`).
- **A new refusal on aeon or Cherub is a STOP.** If any shape stops building because of this check, do not weaken the check and do not edit aeon. Record the routine, line and message, and report BLOCKED on that item. Each is a real defect or a limit of the checker, and the owner decides.
- `Imm` operands are signed `i128` and not masked: mask to 16 bits before reading level bits (`#~$0700` arrives as `Imm(-1793)`).
- The walker's `step` must be pure and monotone. It records nothing. Findings are collected from the returned map after the walk.
- A state has exactly one representation per value (the walker compares with `==` to detect settling).
- No `std::env::` anywhere in `sigil-frontend-emp` (`tests/sandbox_hermeticity.rs` fails on it).
- Tests run with `AEON_DIR` set to a reference aeon at the pinned commit. Totals are reported as the sum of every `test result:` line, never a tail.
- Every new check is proven red first with the mutation quoted from disk, restored from a committed tree, and the red must be an actual red.
- Exact-path `git add` only. Commit each task as it completes, with the finding in the commit body. No co-author trailer.
- No `mcp__oracle__*` tools.

## Review Focus

1. **Unmasked and negative immediates.** `andi.w #~$0700, sr` must read as level 0. Test in Task 2.
2. **A loop that pushes on every pass.** The walk must settle: two paths meeting at different stack depths become "stack lost". Test in Task 2.
3. **Entry in the middle of a routine.** An `export` label and a local helper reached only by `bsr .helper` are both entries at their own caller's level. `QueueDMA_Deferrable` is entered at `.transfer` by two siblings. Tests in Task 3.
4. **The assert rail in DEBUG shapes.** It does `subq #2, sp` and a `d(sp)` store, then leaves and never returns. A lost stack on a path that never returns is not a finding. Test in Task 3.
5. **Code the walk does not reach.** A helper entered through `jsr (a0)`, or a `targets(...)` landing block, that sets the level. Today's line-order check refuses these; the walk must not accept them by never looking. Tests in Task 3.
6. **The saved level thrown away instead of restored.** `addq.l #2, sp` (the W2 plant), `move.w (sp)+, d0`, `move.w (sp)+, ccr`, and a `.l` pop that straddles the saved word. Tests in Tasks 2 and 3.

## Where the answer changes from today

| Shape | Today | After 2a |
|---|---|---|
| A path that masks and leaves without restoring (W2; also through `move.w (sp)+, d0`, under bare `sr`, or by `bra` to the closing label) | builds | refused |
| A write to `sr` before the save or after the last restore, when every path is still in order | refused | builds (correct) |
| A tail or conditional branch out of the routine while masked | builds unless it is the last instruction | refused |
| An `export` label after the save | builds for the routine itself (only entrants are refused) | refused |
| A restore after the stack stopped being followable earlier in the routine (`subq #n, sp`, `link`, a `d(sp)` store) | builds | refused. No aeon routine that makes the promise does this today (0 of 32) |
| A `targets(...)` dispatch inside a masked section, every arm restoring | builds | refused if an arm sets the level (the arms are unreached). A known limit; none in aeon |
| A call to a routine that masks and does not restore | builds | builds (piece 2b) |
| Several findings in one routine | one diagnostic, on the `proc` line | one per way out, on that instruction |

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `crates/sigil-frontend-emp/src/preserves.rs` | add one `pub(crate)` function and enum | Says how one instruction moves the stack pointer, from the helpers the stack check already has |
| `crates/sigil-frontend-emp/src/context.rs` | `edges_for` becomes `pub(crate)` | No behaviour change |
| `crates/sigil-frontend-emp/src/sr_level.rs` | new | The state, its combine rule, the step, the walk and the findings |
| `crates/sigil-frontend-emp/src/lib.rs` | `mod sr_level;` | |
| `crates/sigil-frontend-emp/src/lower/proc.rs` | `check_preserves_sr` calls the walk in place of `sr_writes_round_trip` | Formats the diagnostic |
| `crates/sigil-frontend-emp/tests/sr_level.rs` | new | Whole-source tests through the real front end |
| `docs/superpowers/notes/2026-11-21-stepA-p2a-level-walk.md` | new | Baseline, proofs, transcripts |

`sr_writes_round_trip` stays: the context-definition check (`eval/asm.rs:1018`) still uses it. Nothing in 2a touches that caller.

---

### Task 0: Baseline and measuring equipment

**Files:** Create `docs/superpowers/notes/2026-11-21-stepA-p2a-level-walk.md`.

- [ ] **Step 1:** Confirm A11 is an ancestor of master (`git merge-base --is-ancestor 9f8e1bc6 master`, or confirm by content if it was squashed: `tests/a11_checks.rs` exists on master). If not, STOP.
- [ ] **Step 2:** Create the worktree and branch. Record master's SHA.
- [ ] **Step 3:** Provision reference trees outside the worktree: aeon at the commit Cherub pins or master's paired commit (record which), Cherub at its current head. Reuse `/home/volence/sonic_hacks/.aeon-stepA-p1-ref` and `.cherub-stepA-p1-ref` if their commits still match; record the SHAs.
- [ ] **Step 4:** Build the base binary from master to a run-unique path. Record: `SHAPES` CRCs and sizes for all 7 shapes; the three contract reports saved to files; the front-end test total (`cargo test --release -p sigil-frontend-emp --no-fail-fast`, summed); the last landing count recorded on master.
- [ ] **Step 5:** Record the W2 probes as files under the notes' scratch directory and their results on the base binary: `w2_planted`, `a_leak_pop_d0`, `e_bare_sr_planted` (all three build clean today), and the whole-build plant (`engine/system/dma_queue.emp`, the `.full:` exit's `move.w (sp)+, sr` replaced by `addq.l #2, sp`, in a COPY of aeon). Expected: all accepted. This is the "before".
- [ ] **Step 6:** Commit the notes file.

### Task 1: Say how one instruction moves the stack pointer

**Files:** Modify `crates/sigil-frontend-emp/src/preserves.rs` (new items next to `immediate_sp_cleanup_bytes`, `:882`; tests in its `mod tests`).

**Interfaces, produces:**

```rust
/// How one 68000 instruction moves sp, as far as the stack check's own
/// helpers can say. Built from them so the two readings cannot drift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpEffect {
    /// sp does not move.
    None,
    /// `-(sp)`: this many bytes pushed (a `movem` counts every member).
    Push(u32),
    /// This many bytes leave the stack: `(sp)+`, `rtr`'s condition-code word,
    /// or `add`/`addq`/`adda #n, sp`.
    Drop(u32),
    /// Anything the stack check bails on: a bare `a7`, a `(sp)` store,
    /// `d(sp)` other than the exempt reads, `(sp,Xn)`, `sub #n, sp`, `link`, `unlk`.
    /// Also `lea` or `pea` with an sp-based source: the stack's address escapes.
    Lost,
}
pub(crate) fn sp_effect(items: &[CodeItem], idx: usize) -> SpEffect;
```

Calls (`jsr`/`bsr`/`jbsr`) are `None`, as in `transfer` (`:961`). The exemptions are the ones `transfer` applies at `:1002`: `is_sp_top_read`, `is_safe_sp_disp_read`, `is_irq_frame_access`. Do not change `transfer` or any existing function.

Two stated differences from `transfer`, both because `transfer` gets them wrong for this purpose (MEASURED by the plan's reviewer: `pea (a0)` / `rts` builds with no stack finding today):

- `pea <ea>` is `Push(4)`. `transfer` does not model it. Record this in `docs/QUEUE.md` as a defect of the stack-balance check (entry `STACK-PEA-UNMODELLED`); do not fix it here.
- `lea` or `pea` whose source is `(sp)`, `d(sp)` or `(sp,Xn)` is `Lost`. `transfer` exempts `lea (sp), aN` as a read (`is_sp_top_read`), which lets the saved word be rewritten through `aN`.

- [ ] **Step 1: Write the failing tests** in `preserves.rs` `mod tests`:
  - `sp_effect_reads_each_form`: a table of (instruction, expected): `move.w sr, -(sp)` → `Push(2)`; `move.l d0, -(sp)` → `Push(4)`; `movem.l d0-d2/a0, -(sp)` → `Push(16)`; `movem.w d0-d1, -(sp)` → `Push(4)`; `move.b d0, -(sp)` → `Push(2)`; `pea (a0)` → `Push(4)`; `pea 4(sp)` → `Lost`; `lea (sp), a0` → `Lost`; `move.w (sp)+, sr` → `Drop(2)`; `move.w (sp)+, ccr` → `Drop(2)`; `movem.l (sp)+, d0-d2/a0` → `Drop(16)`; `rtr` → `Drop(2)`; `addq.l #2, sp` → `Drop(2)`; `adda.w #8, sp` → `Drop(8)`; `subq.l #2, sp` → `Lost`; `move.w d0, 1(sp)` → `Lost`; `movea.l 8(sp), a1` → `None`; `move.w (sp), d0` → `None`; `link a6, #-8` → `Lost`; `unlk a6` → `Lost`; `jsr Foo` → `None`; `moveq #0, d0` → `None`.
  - `sp_effect_agrees_with_the_stack_check`: for every row above that is not `Lost` and is not one of the two stated differences (`pea`, `lea`), run the existing private `transfer` on a `State` pre-loaded with enough slots and assert `depth_bytes` moved by exactly the amount `sp_effect` says; for every `Lost` row except `link`/`unlk` and the two stated differences, assert `transfer` returns `Some(_)`.
- [ ] **Step 2:** Run `cargo test --release -p sigil-frontend-emp --lib sp_effect`. Expected: FAIL, `sp_effect` not found.
- [ ] **Step 3:** Implement `SpEffect` and `sp_effect`, in the same order of tests as `transfer` (call, `rtr`, `link`/`unlk`, immediate cleanup, hazard with its three exemptions, push, pop).
- [ ] **Step 4:** Run the same command. Expected: PASS. Then the full front-end suite; total = baseline + 2, 0 failed.
- [ ] **Step 5:** RF: mutate `sp_effect` so the immediate-cleanup arm returns `Push`; quote the mutated line from disk; show `sp_effect_reads_each_form` red; restore with `git checkout <committed sha> -- <path>` after committing the green state. Record in the notes.
- [ ] **Step 6:** Commit.

### Task 2: The state, its combine rule and the step

**Files:** Create `crates/sigil-frontend-emp/src/sr_level.rs`; modify `lib.rs` (`mod sr_level;`).

**Interfaces, consumes:** `context::Lattice`; `preserves::{sp_effect, SpEffect}`; `value::{CodeItem, CodeOperand, Reg, Width}`.

**Interfaces, produces:**

```rust
/// A set of possible interrupt levels. Bit n (0..=7): level n. Bit 8 (`ENTRY`):
/// whatever the level was when the routine was entered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LevelSet(u16);   // field private; never empty (no constructor makes an empty set)
impl LevelSet {
    pub(crate) const ENTRY: LevelSet = LevelSet(1 << 8);
    pub(crate) fn only(level: u8) -> LevelSet;          // level & 7
    pub(crate) fn union(self, o: LevelSet) -> LevelSet;
    pub(crate) fn is_entry_only(self) -> bool;
    pub(crate) fn fixed_levels(self) -> Vec<u8>;        // ascending, ENTRY left out
    pub(crate) fn has_entry(self) -> bool;
}

/// What the level could be at one point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level { Known(LevelSet), Unknown }

/// One saved level on the stack. `below` is the depth in bytes before the push.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Save { pub(crate) below: u32, pub(crate) level: Level }

/// The stack, as far as saved levels need it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Stack {
    /// `depth`: bytes pushed since entry. `saves`: bottom first, strictly rising `below`.
    Known { depth: u32, saves: Vec<Save> },
    Lost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SrState { pub(crate) level: Level, pub(crate) stack: Stack }
impl SrState { pub(crate) fn at_entry() -> SrState; }   // Known(ENTRY), Known { 0, [] }
impl Lattice for SrState { fn meet(&self, other: &Self) -> Self; }

/// The state after the instruction at `idx`, given the state arriving at it.
pub(crate) fn step(items: &[CodeItem], st: &SrState, idx: usize, mnem: &str, ops: &[CodeOperand]) -> SrState;
```

**Rules the code must implement** (this table is the specification; the tests below pin it):

| Instruction | Level | Stack |
|---|---|---|
| `move.w sr, -(sp)` (`[Sr, PreDec(A7)]`) | unchanged | `Known`: push `Save { below: depth, level }`, depth += 2. `Lost` stays `Lost` |
| `move.w (sp)+, sr` (`[PostInc(A7), Sr]`) | if the stack is `Known` and the top save has `below == depth - 2`: that save's level. Otherwise `Unknown` | pop that save if it matched and depth -= 2. If it did not match, apply the `Drop(2)` rule of the last row |
| `move.w #imm, sr` | `Known(only((imm as u16 >> 8) & 7))` | unchanged |
| `move` to `sr` from anything else | `Unknown` | by `sp_effect` |
| `ori.w #imm, sr`, k = `(imm as u16 >> 8) & 7` | each fixed level n → `n \| k`. `ENTRY` → `ENTRY` if k = 0, level 7 if k = 7, else the whole result is `Unknown` | unchanged |
| `andi.w #imm, sr` | n → `n & k`. `ENTRY` → `ENTRY` if k = 7, level 0 if k = 0, else `Unknown` | unchanged |
| `eori.w #imm, sr` | n → `n ^ k`. `ENTRY` → `ENTRY` if k = 0, else `Unknown` | unchanged |
| `ori`/`andi`/`eori` to `sr` with a non-`Imm` source | `Unknown` | unchanged |
| `stop #imm` (mnemonic `stop`, no `Sr` operand) | as `move.w #imm, sr` | unchanged |
| `jsr`/`bsr`/`jbsr` | unchanged (2a; one private function `call_effect` returns this, so 2b changes one place) | unchanged |
| everything else | unchanged | by `sp_effect`: `Push(n)` depth += n; `Drop(n)` depth -= n, then drop every save with `below >= depth`; a save with `below < depth < below + 2` (cut in half) → `Lost`; underflow → `Lost`; `Lost` → `Lost` |

`Level::Unknown` and `Stack::Lost` absorb. An unknown level stays unknown until a `move.w #imm, sr`, a `stop`, an `ori` with k = 7, an `andi` with k = 0, or a restore from a save that holds a known level.

**Combine rule (`meet`):** levels: `Unknown` if either is, else the union. Stacks: `Lost` if either is; else if depths are equal, the save lists have equal length and equal `below` in each position, the save levels combine pairwise by the same level rule; otherwise `Lost`.

- [ ] **Step 1: Write the failing unit tests** in `sr_level.rs` `mod tests`. Build items with a small local helper (model on `preserves.rs:1824`). One test per row, each asserting the full `SrState` after `step`:
  - `an_immediate_move_sets_the_level` (`#$2700` → 7; `#$2300` → 3; `#$2000` → 0)
  - `an_unmasked_negative_immediate_is_read_as_sixteen_bits` (`andi #~$0700` given as `Imm(-1793)` from `Known(only(7))` → 0, and from `ENTRY` → 0)
  - `ori_seven_masks_from_any_level`, `ori_partial_on_the_entry_level_is_unknown` (k = 3 on `ENTRY`), `ori_partial_on_fixed_levels_is_exact` ({2,4} with k = 1 → {3,5})
  - `andi_and_eori_follow_the_bits` (andi k = 7 keeps `ENTRY`; eori k = 0 keeps `ENTRY`; eori k = 7 on {0} → {7}; eori k = 7 on `ENTRY` → `Unknown`)
  - `stop_sets_the_level` (`stop #$2500` → 5)
  - `a_register_source_is_unknown` (`move.w d0, sr`)
  - `save_then_restore_returns_the_saved_level` (ENTRY; save; `#$2700`; restore → ENTRY, depth 0, no saves)
  - `nested_saves_restore_in_order` (save at ENTRY; `#$2700`; save; `#$2300`; restore → 7; restore → ENTRY)
  - `freeing_the_saved_word_discards_it_and_keeps_the_level` (save; `#$2700`; `addq.l #2, sp` → level 7, no saves, depth 0)
  - `popping_the_saved_word_elsewhere_discards_it` (`move.w (sp)+, d0`; and `move.w (sp)+, ccr`)
  - `a_long_pop_that_covers_the_saved_word_discards_it` (`move.w d0, -(sp)`; save; `move.l (sp)+, d1` → the save is dropped whole, depth 0, stack still `Known`)
  - `a_pop_that_cuts_the_saved_word_in_half_loses_the_stack` (save; `addq.l #1, sp` → `Lost`)
  - `a_restore_with_something_pushed_on_top_is_unknown` (save; `move.w d0, -(sp)`; `move.w (sp)+, sr` → `Unknown`: the word popped is not the save)
  - `a_restore_with_a_lost_stack_is_unknown`
  - `a_call_changes_nothing`
  - `meet_unions_levels`, `meet_of_different_depths_loses_the_stack`, `meet_of_equal_stacks_combines_saved_levels`, `meet_with_unknown_is_unknown`, `meet_is_commutative_associative_and_idempotent_on_a_sample` (a fixed list of 8 states, all pairs and all triples)
  - `a_restore_at_an_odd_depth_that_cuts_a_save_loses_the_stack` (save; `addq.l #1, sp`; `move.w (sp)+, sr`)
  - `ori_andi_eori_keep_every_arm` is NOT a shortcut: aeon has 0 such sites today, and the three rows stay exact as tabled (decided: keep, they are in the design)
  - `a_push_in_a_loop_settles`: build a three-instruction loop that pushes each pass, run `forward_in_states` with `SrState::at_entry()` and `step`, assert it returns and the loop head's stack is `Lost`.
- [ ] **Step 2:** Run `cargo test --release -p sigil-frontend-emp --lib sr_level`. Expected: FAIL to compile.
- [ ] **Step 3:** Implement the types, `meet` and `step` from the tables. `debug_assert!` a non-empty set in `meet` and at the end of `step`.
- [ ] **Step 4:** Run again. Expected: PASS. Full front-end suite: baseline + 2 + the count of tests added here, 0 failed. `cargo clippy --workspace --all-targets -- -D warnings` exit 0 (the module is unused outside tests until Task 3: use `#[cfg_attr(not(test), allow(dead_code))]` on the module, and remove it in Task 3).
- [ ] **Step 5:** RF, two mutations, each quoted from disk and each shown red: (a) the restore arm returns the current level instead of the saved one (`save_then_restore_returns_the_saved_level` red); (b) `meet` on different depths returns `self.clone()` (`meet_of_different_depths_loses_the_stack` red, and `a_push_in_a_loop_settles` must still terminate or the mutation is the wrong one). Restore from the committed tree.
- [ ] **Step 6:** Commit.

### Task 3: Walk a routine and judge every way out

**Files:** Modify `crates/sigil-frontend-emp/src/sr_level.rs`; modify `crates/sigil-frontend-emp/src/context.rs` (`fn edges_for` at `:178` becomes `pub(crate) fn edges_for`, nothing else).

**Interfaces, consumes:** `context::{forward_in_states, edges_for}`; `flag_check::{Cfg, Edge}` (and `Cfg::label_index`); `preserves::exit_diverges`; `sigil_ir::backend::Cpu`.

**Interfaces, produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MaskFindingKind {
    /// This way out can be taken at these fixed levels (and perhaps the caller's too).
    NotTheCallers { levels: Vec<u8> },
    /// The level cannot be followed to this way out. `lost_at`: the lowest-index
    /// reached instruction, not authored `AssertDesugar`, whose arriving level is
    /// known and leaving level unknown, or whose arriving stack is known and
    /// leaving stack lost. Its index, mnemonic and span.
    Untracked { lost_at: Option<(usize, String, Span)> },
    /// The routine leaves by `rte`, which loads the level from the stack.
    Rte,
    /// An instruction that sets the level (`sr` as last operand of a writing
    /// mnemonic, `stop`, `rte`) that the walk never reached. The walk cannot
    /// vouch for it: it may be entered through a pointer or a `targets(...)` jump.
    Unreached,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MaskFinding { pub(crate) idx: usize, pub(crate) span: Span, pub(crate) kind: MaskFindingKind }

/// Every way out of a 68000 routine that is not provably at the caller's level.
/// Sorted by `idx`, at most one finding per instruction.
pub(crate) fn mask_findings(items: &[CodeItem], noreturn: &BTreeSet<String>) -> Vec<MaskFinding>;
```

**Rules:**

- **Seeds**, all `SrState::at_entry()`: the first instruction; the instruction after every `CodeItem::Label { export: true, .. }`; the instruction at every label that is the target of a `jsr`/`bsr`/`jbsr` whose `Sym` operand resolves with `Cfg::label_index(name)` (a local helper: it has its own caller and its own return, and it is not entered by the CFG's call edge). Find "the instruction at a label" through `Cfg::label_index` and skip `None` (a body-closing label): `forward_in_states` panics on a seed index past the items. A body with no instruction has no seed and no finding.
- **Walk:** `forward_in_states(&cfg, Cpu::M68000, &seeds, &|st, idx, m, ops| step(items, st, idx, m, ops), &|_| true)`.
- **Judging**, for each reached `idx` in the map, with `out = step(items, in_state, idx, ..)` and each edge of `edges_for(&cfg, Cpu::M68000, idx)`:

| Edge | Verdict |
|---|---|
| `Follow(_)` | nothing |
| `Return`, mnemonic `rte` | `Rte` |
| `Return`, other (`rts`, `rtr`, `rtd`) | judge `out.level` |
| `TailOut`, `BranchOut` | nothing if `exit_diverges(&items[idx], noreturn)`; else judge `out.level` |
| `FallOff` | as `TailOut`. (`check_preserves_sr`'s earlier refusal only looks at the LAST instruction, so a masked `bra .end` to the body-closing label builds today: MEASURED by the plan's reviewer) |

  Judging a level: `Known(s)` with `s.is_entry_only()` → clean. `Known(s)` otherwise → `NotTheCallers { levels: s.fixed_levels() }`. `Unknown` → `Untracked { lost_at }`.
- **Unreached pass:** after judging, every instruction NOT in the result map that is an `sr` write (the test `check_clobbers` uses, `lower/proc.rs:1382`), a `stop` or an `rte` is an `Unreached` finding. Without this the walk would silently accept code today's line-order check refuses (a helper entered through `jsr (a0)`, a `targets(...)` landing block).
- The stack is not judged here. The stack-balance check owns that. A `Lost` stack only matters when it makes a restore unknown.

- [ ] **Step 1: Write the failing tests.** Unit tests in `sr_level.rs` `mod tests` over hand-built items:
  - `the_w2_shape_is_found`: save; `#$2700`; `tst.w d0`; `beq .full`; `addq.l #2, sp`; `rts`; `.full:` restore; `rts` → exactly one finding, on the first `rts`, `NotTheCallers { levels: vec![7] }`.
  - `both_paths_restoring_is_clean`; `a_routine_that_never_touches_sr_is_clean`.
  - `a_remask_in_a_loop_under_one_save_is_clean` (the `Sound_Init` shape: save; `.loop:` `#$2700`; `tst`; `bne .loop`; restore; `rts`).
  - `an_early_exit_before_the_save_is_clean` (the `Sound_DrainSfxRing` shape).
  - `an_export_label_before_the_save_is_an_entry` (the `QueueDMA_Deferrable.transfer` shape: clean) and `an_export_label_after_the_save_is_refused` (an entrant there pops a word it never pushed → the restore is unknown → `Untracked`).
  - `a_local_helper_reached_only_by_bsr_is_walked`: body `bsr .h` / `rts` / `.h:` `move.w #$2700, sr` / `rts` → one finding on the helper's `rts`.
  - `a_register_restore_is_untracked_and_names_where_it_was_lost`: `move.w sr, d1`; `#$2700`; `move.w d1, sr`; `rts` → `Untracked` with `lost_at` = the span of `move.w d1, sr`.
  - `an_rte_is_its_own_finding`.
  - `a_diverging_rail_inside_a_masked_section_is_not_a_finding`: save; `#$2700`; save; `cmp`; `beq .ok`; `subq.l #2, sp`; `move.w d0, 1(sp)`; a `TailOut` instruction authored `ItemAuthor::AssertDesugar`; `.ok:` restore; restore; `rts` → clean. (The rail leaves at level 7 with a lost stack; only the `exit_diverges` skip keeps it clean.)
  - `a_masked_branch_to_the_closing_label_is_found`: save; `#$2700`; `tst`; `beq .full`; `addq.l #2, sp`; `bra .end`; `.full:` restore; `rts`; `.end:` (closing label) → one finding on the `bra`.
  - `a_helper_entered_through_a_pointer_is_unreached`: `lea .h(pc), a0`; `jsr (a0)`; `rts`; `.h:` `move.w #$2700, sr`; `rts` → one `Unreached` finding on the `move`.
  - `an_unreached_rte_and_stop_are_findings`; `unreached_code_that_never_sets_the_level_is_not`.
  - `a_closing_export_label_and_an_empty_body_do_not_panic`.
  - `the_loss_point_skips_assert_lines`: an `AssertDesugar`-authored `subq.l #2, sp` on a rail, then later a hand-written `move.w d1, sr`; `lost_at` names the `move`.
  - `a_tail_out_while_masked_is_found`: save; `#$2700`; `jmp Other` → `NotTheCallers { levels: vec![7] }` on the `jmp`.
  - `a_tail_to_a_noreturn_target_while_masked_is_clean`.
  - `a_conditional_branch_out_while_masked_is_found`.
  - `a_balanced_stop_is_clean` (save; `stop #$2500`; restore; `rts`).
  - `a_return_is_judged_after_its_own_effect`: a last instruction that both changes the state and leaves. Use `stop`-free source: save; `#$2700`; restore; then a `TailOut` `jmp Other` → clean; and the mirror where the restore is missing → found. If no single 68000 instruction in this tree both writes `sr` and carries a leaving edge, say so in the notes and pin `rtr` instead (its `Drop(2)` must not turn a clean return into a finding).
  - `one_finding_per_instruction_sorted_by_index`.
- [ ] **Step 2:** Run. Expected: FAIL to compile (`mask_findings` missing).
- [ ] **Step 3:** Implement `mask_findings`. Make `edges_for` `pub(crate)`. Remove the Task 2 `dead_code` allowance only if the module is now used outside tests; otherwise leave it for Task 4 to remove.
- [ ] **Step 4:** Run. Expected: PASS. Full front-end suite, clippy.
- [ ] **Step 5:** RF, five mutations, each quoted from disk and each red: (a) seed only the first instruction, dropping the local-helper seeds (`a_local_helper_reached_only_by_bsr_is_walked` red); (b) drop the export-label seeds (`an_export_label_after_the_save_is_refused` red); (c) remove the `exit_diverges` skip (`a_diverging_rail_inside_a_masked_section_is_not_a_finding` red); (d) remove the unreached pass (`a_helper_entered_through_a_pointer_is_unreached` red); (e) skip `FallOff` (`a_masked_branch_to_the_closing_label_is_found` red). Restore from the committed tree.
- [ ] **Step 6:** Commit.
- [ ] **Step 7: Ask aeon before wiring.** With the branch committed, apply a TEMPORARY, uncommitted patch that makes `check_preserves_sr` print every `mask_findings` result with `eprintln!` (routine, mnemonic, kind) without raising a diagnostic. No environment variable (the hermeticity test forbids it). Build sonic4 `release` and `debug`, demo `release` and `debug`, and Cherub, and record every printed line in the notes. Expected: none. Then `git checkout <committed sha> -- crates/sigil-frontend-emp/src/lower/proc.rs` and show `git status --short` empty. If any line printed, STOP here and report it (Global Constraints): do not go on to Task 4.

### Task 4: Put the walk in place of the line-order test

**Files:** Modify `crates/sigil-frontend-emp/src/lower/proc.rs` (`check_preserves_sr`, `:2566`, only the third refusal at `:2622`); create `crates/sigil-frontend-emp/tests/sr_level.rs`; adjust existing tests that pin the old message text.

**What changes:** the block `if !sr_writes_round_trip(buf.items.iter()) { … }` is replaced by: for each finding of `crate::sr_level::mask_findings(&buf.items, noreturn)`, one `Level::Error` anchored at `finding.span` (not `proc.span`). The first two refusals (`sr_tail_refusal`, `terminal_external_tail`) are untouched and still return early. `sr_writes_round_trip` itself is not edited.

**Message texts** (exact; `{name}` is the routine, `{token}` is `sr` or `sr.mask`):

- `NotTheCallers`: ``[proc.preserves-sr-unbalanced] `{name}` declares `preserves({token})` but this way out can be reached with the interrupt level at {levels}, not the caller's (a path masks interrupts and does not restore the saved level)`` where `{levels}` is the list joined with ` or `.
- `Untracked` with `lost_at`: ``[proc.preserves-sr-unbalanced] `{name}` declares `preserves({token})` but the interrupt level cannot be followed to this way out: it stops being known at an earlier `{mnemonic}` (restore it with `move.w (sp)+, sr` from the matching `move.w sr, -(sp)`, with the stack as it was at the save)``. Without `lost_at`, the same sentence ending at "this way out". No line number: spans are byte offsets and this function has no source text. If `Level` has a note variant, also push one note anchored at the `lost_at` span, text ``the interrupt level stops being known here``; if it has none, do not add one.
- `Unreached`: ``[proc.preserves-sr-unbalanced] `{name}` declares `preserves({token})` but this instruction sets the interrupt level in code the checker cannot reach from the routine's entries (entered through a pointer or a computed jump), so it cannot be followed``.
- `Rte`: ``[proc.preserves-sr-unbalanced] `{name}` declares `preserves({token})` but leaves by `rte`, which loads the interrupt level from the stack``.

- [ ] **Step 1: Write the failing whole-source tests** in `tests/sr_level.rs`. Harness: start from `tests/a11_checks.rs`, but keep each diagnostic's `primary` span, add a byte-offset-to-line helper in the test file, and for the assert test pass `defines: vec![("DEBUG".into(), 1)]` as `sigil-cli/tests/diag_assert_vector.rs:160` does. Each test is a routine `proc P () clobbers(d0) preserves(sr.mask)` unless stated, asserting the diagnostics' level, the `[proc.preserves-sr-unbalanced]` prefix, the count, and the line the diagnostic is anchored on:
  - `w2_is_refused_at_the_leaking_return` (the review's exact source, quoted in the survey section 7). Asserts one error, on the line of the first `rts`, message containing `at 7`.
  - `w2_through_a_pop_into_d0_is_refused`, `w2_under_bare_preserves_sr_is_refused` (was accepted with only a `[proc.ccr-advisory]` warning).
  - `both_paths_restoring_builds`, `a_routine_that_never_touches_sr_builds`, `a_balanced_stop_builds`, `one_path_remasking_then_restoring_builds` (survey variant `f`).
  - `ori_with_no_save_is_refused`, `a_register_save_is_refused`, `an_rte_is_refused` (survey variants `d`, `h`, `l`: refused before, still refused).
  - `a_call_to_a_masking_routine_still_builds` (survey variant `j`; comment in the test: "2a keeps today's reading of calls; piece 2b closes this", so the test is found and flipped when 2b lands).
  - `with_ints_off_builds` and `with_ints_off_and_an_early_return_inside_is_refused` (pin the FULL set of diagnostics that fire, whatever it is; expected: this check beside `[context.escape]` and `[stack.unbalanced]`).
  - `an_export_label_after_the_save_is_refused_on_its_own`, `a_masked_branch_to_the_closing_label_is_refused`, `a_helper_entered_through_a_pointer_is_refused`.
  - `an_assert_inside_a_masked_section_builds_in_debug` (source with an `assert` between the mask and the restore, built with the debug define the neighbouring tests use).
  - `the_same_text_for_sr_and_sr_mask` (token appears in the message).
- [ ] **Step 2:** Run `cargo test --release -p sigil-frontend-emp --test sr_level`. Expected: the W2 tests FAIL (they build clean today).
- [ ] **Step 3:** Make the change in `check_preserves_sr`. Remove the `dead_code` allowance on the module.
- [ ] **Step 4:** Run the new file. Expected: PASS. Then the full front-end suite. Expected: no existing test needs re-pinning (no test matches the old "not bracketed by" text; every one matches the `[proc.preserves-sr-unbalanced]` prefix). One existing test may gain a second diagnostic (a routine with an `export` label after its save); if a count assertion fails for that reason, record the test's name and both diagnostics and raise the count. If any existing test changes DIRECTION (accepted ↔ refused), STOP on it and report. Also run `cargo test --release -p sigil-cli --test tranche21_spelling_probes --test diag_assert_vector` here: both carry `preserves(sr…)` source.
- [ ] **Step 5:** RF on the wiring: mutate `check_preserves_sr` to skip the findings loop; quote from disk; `w2_is_refused_at_the_leaking_return` red; restore from the committed tree.
- [ ] **Step 6:** Commit.

### Task 5: Whole-tree proof

**Files:** Modify the notes file; modify `docs/QUEUE.md` (one entry, see Step 6).

- [ ] **Step 1 (BN):** Build the candidate binary from the COMMITTED tree (check `sigil --version` shows no `-dirty`). `scripts/step0_shapes_cmp.sh` against the Task 0 baseline: `SHAPES 7/7 IDENTICAL`. If a shape fails to build because of `[proc.preserves-sr-unbalanced]`, that is the Global Constraints STOP: list every firing (routine, file:line, message) in the notes and report.
- [ ] **Step 2 (RN):** `--report contracts` for sonic4, demo and Cherub: three SAME against the Task 0 files.
- [ ] **Step 3 (the W2 proof, whole build):** in a COPY of the reference aeon, quote line 183 of `engine/system/dma_queue.emp` from disk, apply the plant, quote it again, build sonic4 `release`. Expected: exit 1, `[proc.preserves-sr-unbalanced]` naming `QueueDMA_Deferrable`, anchored on the `rts` after the planted line. Restore the copy with `git checkout <sha> -- engine/system/dma_queue.emp`; rebuild; exit 0 and the baseline CRC. Record all four outputs.
- [ ] **Step 4 (debug shapes):** all four of `./build.sh`, `DEBUG=1 ./build.sh`, and both again with `demo`, in the reference aeon with `SIGIL_BUILD`/`SIGIL_EMIT` pointing at the candidate (if those variables are missing, report BLOCKED, do not hunt for a workaround). The debug shapes carry four times the status-register traffic (asserts); they must build.
- [ ] **Step 5 (TC):** full workspace `cargo test --release --no-fail-fast`, summed; `cargo clippy --workspace --all-targets -- -D warnings` exit 0. Expected total: the Task 0 landing baseline plus the `#[test]` lines this branch adds (`git diff <base>..HEAD -- crates | grep -cE '^\+[[:space:]]*#\[test\]'`), 0 failed.
- [ ] **Step 6:** Add to `docs/QUEUE.md` the entry `STACK-PEA-UNMODELLED` (Task 1) and an entry `STEPA-P2B-CALLS`: calls still do not change the level (name the pinned test `a_call_to_a_masking_routine_still_builds`); the 22 non-declaring callees; the two deferred rules; and that `sr_writes_round_trip` survives only for the context-definition check.
- [ ] **Step 7:** Commit. The strict landing run (`scripts/landing-run.sh --baseline N --aeon <copy>`) and the merge are the controller's, after the whole-branch review.

---

## Self-review notes

- Spec coverage: the Piece 2 instruction table is covered except its three call rows (2b, by design, pinned by a test and a QUEUE entry) and `rte` "restores the level from before the interrupt" (2a keeps today's refusal of `rte` under a mask claim; handlers are 2b with piece 3). Rule 1 of "What it then proves" is this plan. Rules 2 and 3 are 2b.
- The design page's "the existing stack walker already pairs pushes with pops" is not usable as written (the walker is private and its slots cannot mark a saved level). Task 1 reuses its helpers instead; Task 2 carries the saves in the new state.
- No baseline rows and no description format change: this is a per-file error, like the check it replaces.
