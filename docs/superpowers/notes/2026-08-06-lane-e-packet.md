# Lane E packet — the Edge out-split: `Defer` → `TailOut` / `BranchOut`

Spec: `docs/superpowers/specs/2026-08-06-lane-e-edge-out-split.md` (Fable, RULED).
Closes the `is_uncond_tail` ledger row (resolved by substance, not by number).
Byte-NEUTRAL parcel. Branch `lane-e`, from sigil master `06f52445` / aeon
master `c29ffbc`. Zero aeon commits.

## What landed

`Edge::Defer` is DELETED. Two variants replace it, so the compiler forced every
consumer arm to choose:

- **`Edge::TailOut`** — the successor of an unconditional transfer that leaves
  the body. Always the instruction's ONLY edge.
- **`Edge::BranchOut`** — the taken successor of a conditional terminator whose
  target is outside the body. NEVER an instruction's only edge.

Neither variant claims anything about the target. That bounding argument is
inherited verbatim into the `Edge` doc so no consumer relitigates it.

The axis is passed into the shared `branch_edge` three-way from the call site as
a private `OutFlavor` — a type rather than a bare `Edge` parameter, so a caller
cannot hand the three-way a local edge as a "flavor". Both of the old `Defer`
arms (external symbol; no-symbol/computed target) map to the caller's flavor.

`djnz`'s taken leg now routes through the same three-way, deleting the raw
`label_target` lookup that silently DROPPED an unresolvable leg. Before: an
external or body-closing `djnz` target produced no edge at all — a path no walk
could see. After: external → `BranchOut` + fall-through; body-closing local →
`FallOff` + fall-through.

`out_verify::is_uncond_tail` is GONE — the fourth mnemonic table of the family
deleted. The two arms read the flavor off the edge.

The stale `Edge` doc claiming a third builder (`z80_preserves`'s "own Z80
builder", which does not exist — it consumes `Cfg::z80_edges`) is fixed.

## Pass breakdown

### Step 3 — retrospect

1. **The spec's §4 blast radius omitted a live consumer file.** `lower/proc.rs`
   consumes `Edge::Defer` at two sites and appears in neither the consumer list
   nor the audited non-consumer list: `check_noreturn`'s escape arm (mechanical
   widening) and `terminal_external_tail`'s singleton test `cfg.edges(last) !=
   vec![Edge::Defer]` (the mask-claim tail credit — a second instance of the
   singleton equivalence the spec pins only for `cycle_budget`). The compiler
   caught both, which is exactly the mechanism the spec's delete-outright ruling
   was chosen for. Handled per §1 semantics; no redesign.
2. **`calls.rs` / `type_slice.rs` / `branch_const.rs` do consume `Edge`, and the
   spec's "no `Edge` consumption" claim is wrong about the first two.** All three
   match only `Edge::Follow` through `let-else`/`if let`, so the CONCLUSION (no
   change needed) holds and they compiled untouched. `branch_const.rs` is named
   nowhere in the spec at all. The spec's strike of `z80_bus.rs` from the
   consumer list is correct — zero `Edge` references, verified.
3. **`is_uncond_tail` had a SECOND caller asking an unrelated question.**
   `is_branch_or_return` (behind `cc_transparent`) used it as a mnemonic-set
   shorthand for a CC-WRITE question. It holds a bare mnemonic and no `Cfg`, so
   it has no edge to read an answer off — not an instance of the family. Of
   `bra|jbra|jmp|jra`, only `jbra` was not already covered by that function's own
   list plus its `b`-prefix rule, so folding in `jbra` is a predicate-equivalent
   change. Ledgered, and flagged as material the ISA-crate classifier row would
   absorb.
4. **Two spec witness citations were imprecise; the third was the brief's, not
   the spec's.** `jbra QueueDMA_Deferrable.transfer` is a bare `Sym` with a
   dotted name, not the `SymOff` the spec calls it (immaterial — both arms take
   the caller's flavor). Those two procs declare `out(carry: dropped)`, a FLAG
   out, so `out_verify`'s register walk never sees them; their `TailOut` is live
   in the `preserves(sr.mask)` terminal-tail credit instead. The commissioning
   brief cited the Z80 witness as `sound_psg.emp:192`, which is
   `PsgVolEnv_Resolve` — a `falls_into` proc, a `FallOff`, not a tail; the SPEC's
   `:252` (`FmVolEnv_Resolve`'s `jr VolEnv_ResolveScan`) is correct.
5. **`ExitKind::Defer` now names a deleted `Edge` variant.** The ruling not to
   split `ExitKind` is followed (one kind absorbs both flavors, doc restated).
   The residual name is ledgered rather than renamed — a rename is mechanical but
   outside what the lane was ruled to change.

### Step 5 — engine-optimize

Nothing. This is a verdict-neutral compiler-internal parcel with no `.emp`
touched and no engine code in scope; the byte bar is the proof.

### Neither bucket — the headline

**The `djnz` routing fix made `cycle_budget`'s `[cycles.ambiguous-branch]`
unreachable, and the strict run caught it.**

The spec called cycle_budget "no behavior change, one equivalence to pin". True
for the corpus, false for the unit surface. `an_unroutable_split_is_refused` —
a `djnz Elsewhere` — asserted `[cycles.ambiguous-branch]`, which fired only
BECAUSE the dropped leg left one edge and the 13/8 split had nowhere to route.
With the leg restored the body earns the structural refusal instead:
`[cycles.unbounded-transfer]`, which names what actually happens — the leg leaves
the proc. Both refuse; the new id is the honest one.

Measured over every split-cost terminator on both CPUs in every target shape
(external, body-closing local, no fall-through; `jr cc`, `djnz`, `ret cc`, 68k
`bXX`/`dbXX`): each now presents exactly two edges. `call cc` does carry a split
cost with one edge, but `[cycles.opaque-call]` refuses it first. So
`BudgetFindingKind::AmbiguousBranch` has no reachable producer.

Disposition: the arm is KEPT (its polarity is safe — refuse, never charge one of
two numbers to the single edge it has) and documented in place as an inputless
defensive guard. The exemplar was NOT deleted — deleting the only exemplar of a
lint is a defect class this campaign has already paid for. It was retargeted to
`a_split_cost_conditional_always_presents_both_its_edges`, a counted 10-shape
sweep pinning the invariant that makes the guard inputless, with a `ret z` twin
that MEASURES. The lint keeps a red-on-regression pin it can no longer fire.

**`BranchOut` has a live 68k corpus witness, in the one arm a `TailOut`-only
widening would have silently dropped.** The spec's §6 prediction — "`BranchOut`
inside an `out()` proc: NONE found" — measured TRUE (0 of 48). But `BranchOut` is
not absent from the corpus: `preserves.rs`'s `AllReturns` tail-clobber marking
sees 5 occurrences, all `player_common.emp:286 bne Player_DebugMove`, against 39
`TailOut`. That arm marks the tail-callee as clobbering what it does not provably
preserve; a `TailOut`-only widening would have dropped it and left a register
`Player_DebugMove` destroys credited as never-written — a `preserves` FALSE
NEGATIVE on a shipping path. Landed correctly. Recorded so the census is never
re-cited as "`BranchOut` has no corpus witness"; the true claim is narrower and
consumer-scoped.

## Census — predictions confirmed / refuted

Measured by instrumenting the consumer arms with `eprintln!`, rebuilding, and
counting; instrumentation removed and the tree verified clean of it afterwards.

| Prediction (spec §6) | Result |
|---|---|
| 68k `TailOut` credit path is LIVE (`Art_Decompress`) | **CONFIRMED** — `out_verify`'s `TailOut` arm executes 48× (12 fixpoint iterations × 4 sites): `jbra ZX0_Decompress`, `jbra S4LZ_Decompress` (both in `Art_Decompress`), and two computed `jmp PcRelIdx` tails inside `S4LZ_Decompress` |
| dma_queue `TailOut` is LIVE | **CONFIRMED, different consumer** — not `out_verify` (their `out` is a carry flag); `terminal_external_tail` fires on `jbra QueueDMA_Deferrable.transfer` ×4 and on `jbra Sound_PostByte` ×12 |
| Z80 `TailOut` is LIVE | **CONFIRMED** — `z80_preserves` transfer-out arm: 217 `TailOut` |
| `BranchOut` inside an `out()` proc: NONE | **CONFIRMED exactly** — 0 of 48 `out_verify` transfer-out executions |
| `djnz` external/trailing: ZERO corpus sites | **CONFIRMED** — 26 `djnz` in the corpus, every one targeting a `.`-prefixed in-body backward local; the instrumented non-`Follow` counter never fired |
| (not predicted) `BranchOut` elsewhere in the corpus | **1 site, 5 observations** — `bne Player_DebugMove`, in `preserves`'s tail-clobber marking (39 `TailOut` / 5 `BranchOut`); 28 `BranchOut` on the Z80 side in `z80_preserves` |

## Bars

1. **Byte bar — BYTE-NEUTRAL, 7/7.** Target list derived from
   `crates/sigil-harness/golden/` (s4, s4.debug, demo, demo.debug, config_a,
   config_b, lean = seven). Full-file CRC / size, post-edit, identical to the
   committed chain-49 goldens and to the pre-edit seed-proof capture:
   `3b6cad91/411167`, `e3963874/423571`, `b8df1c2b/91330`, `30173928/94031`,
   `7660f157/423949`, `ace527ba/301205`, `69c20328/379110`. Anchors likewise
   unmoved. Zero bytes moved.
2. **Full strict — 3509 passed / 0 failed / 4 ignored across 312 binaries.**
   Closing arithmetic: 3509 + 4 = 3513 == this branch's own `#[test]` total
   (3513, counted this session). Master `06f52445`'s total, counted this session,
   is 3506; the +7 delta is eight new test functions minus one rename:
   `a_branch_out_always_has_a_sibling_and_a_tail_out_never_does`,
   `a_computed_transfer_is_a_singleton_tail_out`,
   `a_djnz_leg_that_leaves_the_body_keeps_its_edge`,
   `a_conditional_branch_out_is_not_a_required_return_path`,
   `the_same_target_reached_unconditionally_is_a_required_return_path`,
   `a_conditional_branch_out_does_not_excuse_the_returning_path`,
   `a_djnz_leg_out_of_the_body_is_refused`, and
   `a_split_cost_conditional_always_presents_both_its_edges` (the rename of
   `an_unroutable_split_is_refused`).
3. **`refreeze --check` OK** (tip `ltr-mul`, chain len 49). **`repin --check`:
   `pins.rs unchanged`** — nothing moved, so no 5-site ripple.
   `cargo clippy --release --workspace --all-targets -- -D warnings` clean.
4. **Warn tiers — identical.** Firing lint-id SET is
   `{module.path-mismatch, proc.undeclared-fallthrough, proc.out-unwritten,
   proc.clobber-undeclared}` on all seven shapes, and every per-shape count is
   byte-for-byte identical pre-edit vs post-edit (diffed, not eyeballed). No
   deliberate delta claimed or taken.
5. **Negative probes, both polarities, plus non-vacuity guards.** Four reverts,
   each rebuilt and run:
   - `djnz` leg reverted to the raw `label_target` lookup → the `djnz` pin fails
     (`left: [Follow(1)]`, `right: [BranchOut, Follow(1)]`) AND the sweep pin
     fails.
   - `OutFlavor::Branch → Edge::TailOut` → the sweep's non-vacuity guard fires
     ("the sweep observed only 0 `BranchOut` edges"), 3 tests red.
   - `OutFlavor::Tail → Edge::BranchOut` → the other guard fires ("only 0
     `TailOut` edges"), 4 tests red.
   - `out_verify`'s two arms merged back into one → the `BranchOut`-skip pin
     fails, alone.
   Both sweep assertions carry counted non-vacuity guards (`>= 7` / `>= 6`
   observations), and the cycle_budget sweep asserts its shape count (`== 10`).
   The `BranchOut`-skip pin is bracketed on both sides: the same external target
   reached UNCONDITIONALLY fires, and an unproduced returning path still fires
   with the conditional branch out present.

## Spec pins (§7) — all four

- (a) `BranchOut` is never an instruction's only edge — and `TailOut` always is:
  `a_branch_out_always_has_a_sibling_and_a_tail_out_never_does`, swept over 7
  conditional and 6 unconditional shapes across both CPUs, counted.
- (b) external `djnz` → `[BranchOut, Follow]`; body-closing-local `djnz` →
  `[FallOff, Follow]`; plus the in-body loop unchanged and the no-fall-through
  variant: `a_djnz_leg_that_leaves_the_body_keeps_its_edge`.
- (c) computed transfer → singleton `[TailOut]` on both CPUs:
  `a_computed_transfer_is_a_singleton_tail_out`; `cycle_budget.rs`'s existing
  orthogonality pin updated to `vec![Edge::TailOut]`.
- (d) `out_verify` unit cases: the `TailOut` credit read keeps its three existing
  pins (known producer / non-producer / external), and the `BranchOut` skip gains
  three new ones (skip, unconditional contrast, returning-path still charged).

`Process_DMA_Critical`'s `@budget 670` is unmoved (byte bar + strict both green;
the budget tests pass unchanged).

## Refused / not taken

- The walk-level `falls_into` policy field (spec §5) — out of lane E by ruling;
  that row stays OPEN, unabsorbed.
- The ISA-crate `is_call`/`is_return`/`is_branch` classifier (spec §5) — left
  standing. This parcel deletes one duplicate table and shrinks that row's
  evidence base by one instance; the `is_branch_or_return` finding above adds
  material to it. Noted on the closing row; absorbed nothing.
- Splitting `ExitKind` — declined by the spec, followed.
- Renaming `ExitKind::Defer` — outside the ruled scope; ledgered.
- Deleting the now-inputless `AmbiguousBranch` arm — kept as a defensive guard;
  ledgered with its kill condition.

## Seed proof

The aeon worktree was seeded with six gitignored paths (the boilerplate lists
two). Proven BEFORE any edit: all seven targets built byte-identical to the
committed goldens. Evidence on the four extra seeds: every file in
`engine/debug/generated/` and `engine/sound/generated/` had its mtime advanced to
build time (they are build OUTPUTS), and `build.sh:113-117` auto-builds
`tools/salvador` → `tools/bin/salvador` when missing. So the boilerplate's
two-item list appears sufficient and only `games/sonic4/data/editor/` is
genuinely required — evidence, not proof, since a bare worktree was not tested.
