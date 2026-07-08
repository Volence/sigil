# Plan 7 #7-pre — the L-H.1 final-size placement fix: implementation notes

Worktree `/home/volence/sonic_hacks/sigil/.worktrees/plan7-item7-banks`, branch
`plan7-item7-banks`. Plan:
docs/superpowers/plans/2026-07-08-spec2-plan7-item7pre-placement-fix.md.
RED evidence recorded per task, per the 2026-07-08 here-fix / item9b precedent.

## T0 — baseline probes

Verified on master `dfe6e7b` (the main checkout at `/home/volence/sonic_hacks/sigil`,
clean, on `master`, matching this worktree's fork point):

- `cargo test --workspace --no-fail-fast` → EXACTLY 4 failing tests, all
  allowlisted upstream aeon strlen drift, zero others:
  - `full_build_reproduces_sound_driver_regions`
  - `vector_table_matches_reference_rom_first_256_bytes`
  - `full_debug_rom_matches_assembled_reference`
  - `full_rom_matches_assembled_reference`
- `cargo clippy --workspace --all-targets -- -D warnings` → clean (no
  warnings, no errors).

Both re-confirmed independently in the worktree (T0, 2026-07-08, HEAD =
`78b0655`, content-identical to master `dfe6e7b` apart from the two plan
docs): same 4 named reds, nothing else; clippy clean.

### `scripts/corpus_bytediff.sh` — the probe

New script, plain bash (`set -u`, no cleverness). Builds `sigil-cli` in BOTH
this worktree and the pristine master checkout
(`/home/volence/sonic_hacks/sigil`), then runs each tree's own
`target/debug/sigil` binary against the SAME source files (the worktree's
copies) for:

- every `examples/*.emp` single-file build (`sigil emp <f> -o <tmp>`), and
- the two standing game invocations (`--root examples/game --prelude
  prelude`) for `examples/game/badniks/pitcher_plant.emp` and
  `examples/game/badniks/pitcher_plant_script.emp`.

Each pair is byte-diffed with `cmp`. Verdict per file: `IDENTICAL` /
`DIFFERS` / `SKIPPED` (master's binary failed to compile that file — does
not affect exit status). Exits nonzero iff any file `DIFFERS`.

### T0 run (worktree == master content-wise; sanity that the probe itself works)

```
== single-file examples (examples/*.emp) ==
SKIPPED  composition_pitcher_plant.emp (master's binary failed to compile it)
IDENTICAL dispatch.emp
IDENTICAL guards.emp
SKIPPED  main.emp (master's binary failed to compile it)
IDENTICAL offset_table.emp
IDENTICAL prelude.emp
IDENTICAL reach_branches.emp
IDENTICAL sst_overlay.emp
== game invocations (--root examples/game --prelude prelude) ==
IDENTICAL pitcher_plant.emp
IDENTICAL pitcher_plant_script.emp
RESULT: all identical (SKIPPED files, if any, excluded)
EXIT=0
```

`composition_pitcher_plant.emp` and `main.emp` are pre-existing corpus
failures on master itself (unrelated to this branch — `unknown name
\`timer\`` / undeclared-fallthrough diagnostics, and a missing type
annotation on `ObjectIndex`, respectively), so both binaries fail on them
identically and the probe correctly reports `SKIPPED` rather than
`DIFFERS`. All 8 buildable targets (6 single-file + 2 game) are
byte-identical, confirming the probe works before this branch touches any
placement code.

## RED evidence (filled in per task)

| Task | Test | RED command | RED result | GREEN commit |
|------|------|-------------|------------|---------------|
| 1 | `single_file_growth_overlap_is_fixed` (`crates/sigil-cli/tests/placement_fix.rs`) | `cargo test -p sigil-cli --test placement_fix` | FAILED — first-diff: `left: [78, 249, 0, 0, 222, 173, 190, 239]` (master, 8B = `4E F9 00 00 DE AD BE EF`, `data`'s `DE AD` clobbered the grown jmp operand) vs `right: [78, 249, 0, 0, 128, 0, 222, 173, 190, 239]` (correct, 10B = `4E F9 00 00 80 00 DE AD BE EF`). CLI run on the source confirms master image byte-for-byte. | GREEN at the Task-4 commit below. |
| 4 | `final_placement::{chained_successor_follows_grown_predecessor_final_size, colliding_pins_are_a_loud_link_error, placement_growth_feeds_relaxation_growth_to_a_joint_fixpoint}` (`crates/sigil-link/tests/final_placement.rs`) | `cargo test -p sigil-link --test final_placement` | FAILED (3 of 4) — (a) chained successor `left: 4` vs `right: 6` (baked baseline not the grown final); (c) `unwrap_err()` on `Ok` (no overlap check yet); (d) `left: 4` vs `right: 6` (no placement pass). Test (b) max-span degeneracy passed pre-impl (input lmas already correct). | GREEN at the Task-4 commit below (all 4). |

## T4 — the link-time placement pass + placement⇄relaxation joint fixpoint

**Seam chosen (R7p.3, recommended zero-API-churn variant).** Placement is folded
INTO `resolve_layout`'s existing fixpoint loop (`crates/sigil-link/src/relax.rs`),
so `resolve_layout`'s signature is UNCHANGED and every caller (single-file CLI
tail, multi-module tails, harness, all the direct-`resolve_layout` tests) inherits
the fix with zero wiring. Loop shape per outer pass:

0. `place_pass` (R7p.2): walk sections in vec order with a per-`group` cursor;
   `Pinned` → base = `sec.lma` (its baked anchor), reset the group cursor to it;
   `Chained` → base = the group cursor; advance the cursor by
   `max(reserved_span, final_size(sec, rungs))`. Rewrites `sec.lma` on a mutable
   `placed: Vec<Section>` clone; returns whether any lma moved. The
   `// #7-main: bank bump seam (D7.2)` marker sits between choosing `base` and
   advancing the cursor.
1. (a) rebuild the symbol table from the (possibly moved) origins; (b) the
   existing rung-selection sweep (grow-only, persisted across passes).
2. Converged iff `!grew && !moved`. Then (c) the ladder convergence sweep, (c2)
   the R7p.4 overlap check over every non-empty placed `[lma, lma+final_size)`
   pair, (d) lower + rebuild carrying the placed lmas.

Cap = `(total_flips + 2).max(64)` — the flips bound plus one placement-settle
pass, floored at the ruling's 64 honesty backstop; the non-convergence `Err` is
unreachable by the grow-only/deterministic argument.

`final_size(sec, rungs)` = `placement_span`'s cursor replay but counting
relaxables at their CURRENT rung width (`frag_len`) and honoring `Org` extent
identically. `overlap_diag` names both sections + both hex extents.

**AS-provenance fold-in (R7p.1, Task-5 core, landed early).** The R7p.3 seam
activates the AS/mixed direct-`resolve_layout` callers, which surfaced the AS
`org`-jump provenance gap: an `org` that jumps the physical counter
(`directive_org`, both the closed-section and forward-past-extent arms) opens a
section whose baked lma is an intentional GAP, but the builder default stamped it
`Chained` → the placement pass compacted the gap (`asl_snippets::org_forward_new_section`
diverged: `[1,2,3,4,5,6]` vs golden `[1,2,3,4, 0×12, 5,6]`). Fix per R7p.1: a new
`IrBuilder::pin_next_section` (consumed by the next `switch_section*`) is called
from `directive_org`'s two counter-jump sites, so an org'd section is `Pinned` at
its counter. Naturally-chained AS sections stay `Chained` (growth still reflows
them, matching asl) — the harness s4/m1b/m1c placement-sensitive greens are the
degeneracy proof.

**Mixed-build ports tests (`crates/sigil-cli/tests/ports.rs`).** The two
`mixed_build_cross_seam_*` tests manually concatenate two independently-lowered
modules (each first section `Pinned` at lma 0) and call `resolve_layout` directly
with NO placer — R7p.4 correctly flags the two Pinned-at-0 ranges as colliding
pins. Fixed by calling `place_sequential(&mut sections, 0)` first (mirroring
production `build_program`'s no-map tail); the cross-seam symbol resolves from the
emp section's VMA, so its LMA is irrelevant. R7p.4 left un-weakened.

### Verification ladder (all green)

- (i) `cargo test -p sigil-link --test final_placement` → 4/4 ok.
- (ii) `cargo test -p sigil-cli --test placement_fix` → `single_file_growth_overlap_is_fixed` ok (Task-1 RED → GREEN).
- (iii) `cargo test --workspace --no-fail-fast` → EXACTLY the 4 allowlisted
  sigil-harness reds (`full_build_reproduces_sound_driver_regions`,
  `vector_table_matches_reference_rom_first_256_bytes`,
  `full_debug_rom_matches_assembled_reference`,
  `full_rom_matches_assembled_reference` — all pre-existing aeon-tree/strlen
  drift, unchanged); nothing else red. `module_resolution` 42/42 (gap pins hold),
  `sigil-frontend-emp` all green, `m1b_gate` 5/5.
- (iv) `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- (v) `bash scripts/corpus_bytediff.sh` → `RESULT: all identical`.

## T5 closure (controller, 2026-07-08)

Task 5's substance landed inside T4's commit 20ace91 (implementer-flagged deviation, spec-review-verified):
the AS front-end inherits first-Pinned/rest-Chained provenance from the IrBuilder default (one builder per
program run), and `directive_org`'s two counter-jump sites pin org'd sections (`pin_next_section`) so
intentional gaps survive placement. No further AS provenance work found. Evidence: full harness from the
worktree = EXACTLY the 4 allowlisted reds (aeon strlen drift); spec reviewer independently diffed against
parent 1b18ce6 and confirmed zero new reds / byte-identical placement-sensitive pins (m1b_gate 5/5,
m0_regions, m1c). T4 quality review: APPROVED, 4 minors (none blocking); carry-forward constraint for
#7-main: do NOT add a fifth cursor-replay loop for the bank bump — extract the ir replay primitive first
(rule-of-three already at four: vma_len / placement_span / image_bytes / final_size).
