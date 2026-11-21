---
kind: plan
status: approved
description: "Step A, piece 1: the one reusable path walker in the .emp front end stops being fixed to three values and can carry any state a check defines. The two checks that use it move over and report exactly what they report today. Sigil only; no ROM byte and no diagnostic changes."
---
# Step A Piece 1: Shared Walker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **AEON PAIR: NOT NEEDED.** This piece changes no accepted input, no emitted byte and no diagnostic. Aeon and Cherub are read only, as build inputs for the proofs.

**Goal:** `context::must_in_states`, today fixed to the three-value `Tri`, becomes a walker generic over any state type that says how two states combine at a join, and both of its callers produce byte-identical ROMs and character-identical contract reports.

**Architecture:** One new trait, `Lattice`, and one generic function, `forward_in_states`, replace `must_in_states` in `crates/sigil-frontend-emp/src/context.rs`. `Tri` implements `Lattice` with today's meet. The two callers (`context::check_regions`, `z80_bus::check_bus_state`) call the generic form. Nothing else in the tree is rewritten: the seven other private worklists stay where they are.

**Tech Stack:** Rust workspace pinned to 1.99.1 (`rust-toolchain.toml`); `sigil build --project <game dir> --shape <row> [--report contracts]`; `scripts/step0_shapes_cmp.sh`; `scripts/provision-aeon-ref.sh`.

**Spec:** `/home/volence/sonic_hacks/empyrean/docs/v2/specs/2026-11-21-step-A-machine-state-checker-design.md`, section "Piece 1. One walker that can carry any state". Read the whole page; it is short. Piece 2 of that page (interrupt level) is the first real consumer and is **not** in this plan.

## Global Constraints

- Branch `parcel/stepA-p1-shared-walker`, worktree `/home/volence/sonic_hacks/.sigil-stepA-p1`, cut from sigil `master` at the tip current when Task 0 runs (record the SHA; this plan's line numbers are as of `1e200e1b`). Run `git branch --show-current` before every commit. Never commit to master. Never touch `/home/volence/sonic_hacks/sigil` (the main checkout), `/home/volence/sonic_hacks/.sigil-a10`, or any other worktree.
- **Files this plan may change:** `crates/sigil-frontend-emp/src/context.rs`, `crates/sigil-frontend-emp/src/z80_bus.rs`, `docs/QUEUE.md`, and one new notes file `docs/superpowers/notes/2026-11-21-stepA-p1-shared-walker.md`. Anything else is STOP and BLOCKED. In particular do not touch `eval/asm.rs`, `eval/mod.rs`, `lower/code.rs`, `parser.rs`, `resolve/contract.rs` or `sigil-cli/src/main.rs`: task A10 is editing them on another branch.
- **Not byte-changing, not diagnostic-changing.** Every task that changes code ends with protocols **BN** and **RN** passing. A byte or a report line that moves is STOP and BLOCKED: never adjust a check to get back to green.
- **Not in this piece** (spec, verbatim): "moving the other seven private walking loops over. That is worth doing, but each is its own risk, and none is needed for pieces 2 to 5. It is recorded as owed." Task 3 books it in `docs/QUEUE.md`.
- No new diagnostic, no new `pub` item (everything added is `pub(crate)`), no new dependency.
- Never build inside a live aeon or cherub checkout. Reference trees are provisioned in Task 0, outside every checkout.
- Build with `CARGO_TARGET_DIR=.target-land` in the worktree. Tests need `AEON_DIR=<the aeon reference tree> ORACLE_DIR=/home/volence/sonic_hacks/oracle-old`.
- `df -h /` before a multi-shape build; under 60 GB free, do not run it: BLOCKED.
- Before reporting a task: `CARGO_TARGET_DIR=.target-land cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- Never use an `mcp__oracle__*` tool. Exact-path `git add`; `git show --stat` after each commit; the finding goes in the commit body; no co-author trailer. Use `git commit -F <file>` for any message containing backticks.
- Run every command in the foreground. Never end a turn waiting on a background notification.
- If blocked on any item, stop on that item, write exactly why in the notes file, commit the notes, and continue with what does not depend on it.

## Shared protocols

**BN (byte-neutral).** After `CARGO_TARGET_DIR=.target-land cargo build --release -p sigil-cli`:

```bash
scripts/step0_shapes_cmp.sh --base "$BASE" --cand .target-land/release/sigil --aeon-ref "$AEON_REF"
```

Expected last two lines: `SHAPES 7/7 IDENTICAL` and `STEP0_CMP_END rc=0`. Exit 1 = a byte moved (STOP). Exit 2 = could not run, never a pass.

**RN (report-neutral).** For each of the three rows below, run base and candidate and `diff` the two outputs. Expected: `diff` prints nothing and exits 0 for all three, and each output file is non-empty (`wc -l` shown).

| Row | Command tail |
|---|---|
| sonic4 | `build --project "$AEON_REF/games/sonic4" --shape release --report contracts` |
| demo | `build --project "$AEON_REF/games/demo" --shape release --report contracts` |
| cherub | `build --project "$CHERUB_REF" --shape release --report contracts` |

An empty output, a failed run on either side, or a missing file is "did not run", never a pass.

**TC (touched crate).** `AEON_DIR="$AEON_REF" ORACLE_DIR=/home/volence/sonic_hacks/oracle-old CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --no-fail-fast`, quoting the **sum** of every `test result:` line, never a tail.

**RF (red-first with the mutation shown).** (1) `git status --short` shows no tracked change. (2) Apply the named one-line mutation; show `git diff --stat` and `grep -n` of the mutated line. (3) Run the named test: it must FAIL on the named assertion. Still green means a stale binary or the wrong target dir: fix the runner, do not claim the proof. (4) `git restore --source=HEAD -- <the file>`; `git status --short` clean; re-run: PASS.

## Review Focus

1. **A join where the two incoming states differ must re-queue the successor exactly as today.** The loop re-queues when the merged state differs from the stored one. If the generic form compares the wrong pair, a bus state that should fall to `Unknown` stays definite and `[bus.*]` fires where it must not. Test in Task 1 (`join_of_unequal_states_requeues`) and the existing `tests/z80_bus.rs` join test.
2. **A state type whose meet does not settle would loop forever.** The trait's contract is that repeated meets reach a fixed point; `forward_in_states` documents it. Test in Task 1 (`loop_reaches_fixpoint_on_a_counting_state`): a back edge with a saturating state must terminate.
3. **`follow` must still stop a walk at a region boundary.** `check_regions` relies on an edge leaving the region being absent from the result map. Test in Task 1 (`follow_false_stops_propagation`) and the existing `tests/context_brackets.rs`.
4. **Z80 routines take the Z80 successor model.** `edges_for` picks it from `cpu`. Test in Task 1 (`z80_cpu_uses_z80_edges`).
5. **A seed on an index that is not an instruction** (a label or data item) is skipped, not a panic, as today (`let Some((mnem, ops)) = cfg.instr(idx) else { continue }`). Test in Task 1 (`seed_on_non_instruction_is_inert`).

---

### Task 0: Worktree, base binary, reference trees, baseline

**Files:**
- Create: `docs/superpowers/notes/2026-11-21-stepA-p1-shared-walker.md`

**Interfaces:**
- Produces: shell variables every later task uses, recorded in the notes file with their values: `BASE` (path to a release `sigil` built from the cut point, in a detached worktree outside every checkout), `AEON_REF` (aeon reference tree), `CHERUB_REF` (a `cp -a` copy of `/home/volence/sonic_hacks/cherub` outside every checkout, `build/` output removed).

- [ ] **Step 1:** From the main sigil checkout, read only: `git -C /home/volence/sonic_hacks/sigil rev-parse master`. Create the worktree and branch from that SHA at `/home/volence/sonic_hacks/.sigil-stepA-p1`. Record the SHA.
- [ ] **Step 2:** Build `BASE`: a second detached worktree of the same SHA at `/home/volence/sonic_hacks/.stepA-p1-base`, `cargo build --release -p sigil-cli` there. `BASE` is its `target/release/sigil`. Show `$BASE --version`.
- [ ] **Step 3:** Provision `AEON_REF` with `scripts/provision-aeon-ref.sh /home/volence/sonic_hacks/.aeon-stepA-p1-ref <rev>`, where `<rev>` is the aeon revision sigil's goldens pin names at the cut point (read it from the script's own output or `docs/`; quote where you read it). Exit 0 required.
- [ ] **Step 4:** Make `CHERUB_REF` as described under Interfaces. Check which aeon and sigil it pins (`ci/*.pin`) and record them; if its pinned aeon differs from `AEON_REF`'s revision, record that and use the pinned one for the cherub row only.
- [ ] **Step 5:** Run **BN** with `--cand "$BASE"` (base against base). Expected `SHAPES 7/7 IDENTICAL`. Run **RN**'s three commands with `$BASE` only and save the outputs as `rn-base-{sonic4,demo,cherub}.txt` in a scratch directory outside the worktree; show `wc -l` of each.
- [ ] **Step 6:** Run **TC** on the untouched branch. Record the summed totals: this is the number Task 2 must match plus Task 1's new tests.
- [ ] **Step 7:** Write the notes file with every value above, the wall-clock time of each run, and commit it.

### Task 1: `Lattice` and `forward_in_states`, proven on a state that is not `Tri`

**Files:**
- Modify: `crates/sigil-frontend-emp/src/context.rs` (add beside `must_in_states`, which stays untouched in this task; add a `#[cfg(test)] mod walker_tests` at the end of the file)

**Interfaces:**
- Produces:

```rust
/// A state a forward walk can carry. `meet` combines the states of two paths
/// arriving at one instruction. It must be commutative, associative and
/// idempotent, and repeated meets from any start must reach a fixed point in
/// finitely many steps; the walk terminates only because of that.
pub(crate) trait Lattice: Clone + PartialEq {
    fn meet(&self, other: &Self) -> Self;
}

/// The per-instruction IN-state fixpoint: forward over the shared CFG from
/// `seeds`, transfer `step`, join = `Lattice::meet`, worklist to a fixpoint.
/// `step` receives the instruction's index as well as its mnemonic and
/// operands. `follow` gates edge propagation, as in `must_in_states`.
pub(crate) fn forward_in_states<S: Lattice>(
    cfg: &Cfg,
    cpu: Cpu,
    seeds: &[(usize, S)],
    step: &dyn Fn(&S, usize, &str, &[CodeOperand]) -> S,
    follow: &dyn Fn(usize) -> bool,
) -> BTreeMap<usize, S>;

impl Lattice for Tri { /* today's tri_meet */ }
```

- [ ] **Step 1: Write the failing tests** in `walker_tests`. Build `CodeItem`s by hand, the way `flag_check.rs`'s `edge_model_tests` does (`instr(mnemonic, ops)`, `label(name)`, `sym(n)`; copy those three helpers). Use a test-only state:

```rust
/// A set of small numbers, as a bitmask. Meet is union. Eight bits, so finite.
#[derive(Clone, PartialEq, Debug)]
struct Levels(u8);
impl Lattice for Levels {
    fn meet(&self, o: &Self) -> Self { Levels(self.0 | o.0) }
}
```

Tests, each asserting on the returned map:

| Test | Routine | Seed and step | Assert |
|---|---|---|---|
| `straight_line_carries_a_number` | `moveq`, `moveq`, `rts` | seed `Levels(1)`; step shifts left by one on `moveq` | IN at `rts` is `Levels(0b100)` |
| `join_of_unequal_states_requeues` | `beq .a`, `moveq`, `.a:`, `nop`, `rts` | seed `Levels(1)`; `moveq` sets `Levels(2)`, others identity | IN at `nop` and at `rts` is `Levels(0b11)` |
| `loop_reaches_fixpoint_on_a_counting_state` | `.top:`, `moveq`, `bne .top`, `rts` | seed `Levels(1)`; `moveq` does `Levels((s.0 << 1) | s.0)` | returns; IN at `rts` is `Levels(0xFF)` |
| `follow_false_stops_propagation` | `nop`, `nop`, `rts` | identity step; `follow` is `\|i\| i < idx_of_second_nop` | map has the first `nop`, lacks the second and `rts` |
| `z80_cpu_uses_z80_edges` | a Z80 `jr nz, .a`, `nop`, `.a:`, `ret` | `Cpu::Z80`, identity step, seed `Levels(1)` | map contains `ret`'s index and the `nop`'s |
| `seed_on_non_instruction_is_inert` | `.l:`, `rts` | seed placed on the label's index | returns without panic; map holds only the seed entry |
| `step_receives_the_instruction_index` | `nop`, `nop`, `rts` | step records every `idx` it is given into a `RefCell<Vec<usize>>` | recorded indices equal the three instruction indices, in order |
| `tri_meet_through_the_trait` | none | none | `Tri::Held.meet(&Tri::Held) == Tri::Held`; `Tri::Held.meet(&Tri::NotHeld) == Tri::Unknown`; `Tri::Unknown.meet(&Tri::Held) == Tri::Unknown` |

- [ ] **Step 2: Run to verify they fail.** `CARGO_TARGET_DIR=.target-land cargo test --release -p sigil-frontend-emp --lib walker_tests`. Expected: does not compile (`Lattice`, `forward_in_states` not found).
- [ ] **Step 3: Implement** `Lattice`, `impl Lattice for Tri` (delegating to `tri_meet`), and `forward_in_states`, as a copy of `must_in_states`'s loop with the state made generic and `idx` passed to `step`. `must_in_states` itself is not changed in this task.
- [ ] **Step 4: Run to verify they pass.** Same command. Expected: 8 passed.
- [ ] **Step 5: RF** on `join_of_unequal_states_requeues`. Mutation: in `forward_in_states`, replace the `Some(existing) => existing.meet(&st_out)` arm's result with `existing.clone()`. Expected red: the assertion on `Levels(0b11)`.
- [ ] **Step 6:** Clippy, then commit `context.rs`. Body: what was added, the 8 test names, and the RF result with the mutated line quoted.

### Task 2: Move both callers onto `forward_in_states`; delete `must_in_states`

**Files:**
- Modify: `crates/sigil-frontend-emp/src/context.rs` (`check_regions`, about line 362; the module header comment; delete `must_in_states`)
- Modify: `crates/sigil-frontend-emp/src/z80_bus.rs` (`use` at line 49; the call at line 224; the header comment at line 13)

**Interfaces:**
- Consumes: `forward_in_states`, `Lattice`, `impl Lattice for Tri` from Task 1.
- Produces: no `must_in_states` anywhere in the crate. `tri_meet` stays as the private function `Tri`'s `meet` calls.

- [ ] **Step 1:** In `check_regions`, call `forward_in_states` with the seed `Tri::Held`, step `&|st, _, _, _| *st`, and the same `follow`. In `check_bus_state`, call it with step `&|st, _, mnem, ops| step(*st, mnem, ops)` and the same seed and `follow`. Result types and every use of the maps stay as they are.
- [ ] **Step 2:** Delete `must_in_states`. `grep -rn "must_in_states" crates/` must print nothing; update the three doc comments that name it (`context.rs` header and the `Region` doc near line 305, `z80_bus.rs:13`) to name `forward_in_states`. Keep the header's statement that the other nets still carry their own worklists.
- [ ] **Step 3:** **TC.** Expected: the Task 0 total plus 8, 0 failed. `tests/z80_bus.rs` and `tests/context_brackets.rs` in particular: quote their two `test result:` lines.
- [ ] **Step 4:** **BN.** Expected `SHAPES 7/7 IDENTICAL`.
- [ ] **Step 5:** **RN.** Expected three empty diffs against the Task 0 outputs, each file non-empty.
- [ ] **Step 6: RF, to prove the callers really run the new function.** Mutation: in `forward_in_states`, make the `follow` test `if follow(succ) { continue; }` (inverted). Named test: `cargo test --release -p sigil-frontend-emp --test z80_bus double_stop_fires`. Expected red: with `|_| true` as `follow`, nothing propagates past the entry, so the second stop is never seen as provably stopped and the count assertion fails.
- [ ] **Step 7:** Clippy, then commit both files. Body: the before and after test totals, the BN line, the three RN `wc -l` figures, and the RF result.

### Task 3: Book what is owed, and close the notes

**Files:**
- Modify: `docs/QUEUE.md`
- Modify: `docs/superpowers/notes/2026-11-21-stepA-p1-shared-walker.md`

**Interfaces:**
- Consumes: nothing.
- Produces: one `docs/QUEUE.md` entry, in that file's existing entry style, titled "Seven private worklists still walk `flag_check::Cfg` on their own", naming them (`calls`, `out_verify`, `type_slice`, `branch_const`, `z80_preserves`, `z80_out_verify`, `flag_check`), pointing at `forward_in_states`, and stating that step A pieces 2 to 5 do not need them moved.

- [ ] **Step 1:** Confirm the seven names by `grep -ln "VecDeque" crates/sigil-frontend-emp/src/*.rs`; if the list differs from the seven above, write the list you found and say so.
- [ ] **Step 2:** Add the queue entry. Finish the notes file: branch tip SHA, every protocol result with wall-clock time, and anything found that this plan did not expect.
- [ ] **Step 3:** Commit both files. Report: branch, tip SHA, files changed, summed test totals before and after, BN and RN results, both RF results, and anything left open.

## After the last task

One fresh reviewer reads the whole branch against this plan and the spec section, with the diff and not this session's conclusions. Landing to master is the overseer's act, with the owner's go-ahead, after A10's landing if A10 is mid-landing at the time (the files do not overlap, so order does not matter for content, only for the landing log).
