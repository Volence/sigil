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

**The `djnz` routing fix made `cycle_budget`'s `AmbiguousBranch` VARIANT
unreachable, and the strict run caught it. The lint ID stays live.**

The spec called cycle_budget "no behavior change, one equivalence to pin". True
for the corpus, false for the unit surface. `an_unroutable_split_is_refused` —
a `djnz Elsewhere` — asserted `[cycles.ambiguous-branch]`, which fired only
BECAUSE the dropped leg left one edge and the 13/8 split had nowhere to route.
With the leg restored the body earns the structural refusal instead:
`[cycles.unbounded-transfer]`, which names what actually happens — the leg leaves
the proc. Both refuse; the new id is the honest one.

The ID is NOT dead: `[cycles.ambiguous-branch]` has a second, live, pinned
producer — `z80_cycles.rs`'s `CycleBail::AmbiguousBranch`, reached through the
`cycles(L1, L2)` span builtin (`eval/builtins.rs`) and pinned by `z80_cycles`'s
own `mod tests` and `tests/t40_cycles.rs`. Only `BudgetFindingKind::AmbiguousBranch`
is inputless, and it has TWO producers, inputless for DIFFERENT reasons:

- the `Split && !two_way` guard — every split-cost terminator on both CPUs
  presents exactly two edges (enumerated: `jr cc`, `djnz`, `ret cc`, 68k
  `bXX`/`dbXX`, in external / body-closing-local / no-fall-through shapes);
- the enumerated-dispatch arm — a dispatch mnemonic never carries a Split table
  cost, so its `WalkCost::Split` case has no input either.

The near miss, and it is an ORDERING dependency: `call cc` DOES carry a split
cost (17/10) over a single edge. It is saved only because the call bail is the
FIRST refusal `charged_edges` makes. DEMONSTRATED this session — move the bail
below the `two_way` guard and `call nz, Helper` earns `[cycles.ambiguous-branch]`.
The evidence for inputlessness is a counted ENUMERATION of today's terminator
shapes, not a proof: a shape added later is not in the sweep.

Disposition: the arm is KEPT (its polarity is safe — refuse, never charge one of
two numbers to the single edge it has) and documented in place with its KILL
CONDITION. The exemplar was NOT deleted — deleting the only exemplar of a lint is
a defect class this campaign has already paid for. It was retargeted to
`a_split_cost_conditional_is_refused_before_its_edges_are_counted`, whose name
now states what it actually asserts: all ten of its shapes are refused by the
STRUCTURAL transfer-out loop, which fires on the first leaving edge and returns
BEFORE `two_way` is computed, so a one-edge shape would earn the same id and the
sweep would stay green. (Verified by hand: drop the `djnz` fall-through push and
the sweep is still green.) The edge-COUNT invariant is therefore pinned crate-side
instead, off the edge builders themselves —
`a_split_cost_terminator_presents_exactly_two_edges` in `cycle_budget.rs`'s
`mod tests`, which FAILS under that same revert. `a_call_is_refused` gained the
`call nz, Helper` case, so the ordering dependency is pinned too.

**`BranchOut` has TWO live 68k corpus witnesses, and the widening that keeps them
is correct on POLARITY grounds — no false negative is demonstrable at either.**
The spec's §6 prediction — "`BranchOut` inside an `out()` proc: NONE found" —
measured TRUE (0 of 48). But `BranchOut` is not absent from the corpus:
`preserves.rs`'s `AllReturns` tail-clobber marking sees it at two sites —
`games/sonic4/player/player_common.emp:286 bne Player_DebugMove` (inside
`Player_Main`, `preserves(d7.w)`) and `games/sonic4/objects/test_player.emp:129
bne TestPlayer_Debug` (inside `TestPlayer_Main`, `preserves(a0, d7.w)`).

Counts are SHAPE-SCOPED, measured this session by instrumenting the arm and
rebuilding every target: plain s4 / config_b / lean = **5 observations, one site**
(`Player_DebugMove`); s4.debug / config_a = **14, both sites**
(`Player_DebugMove` 5, `TestPlayer_Debug` 9 — nearly double, because
`preserves(a0, d7.w)` drives both the Full and the Word facet where
`preserves(d7.w)` drives only the Word); demo / demo.debug = **0**. Against 39
`TailOut` (plain) / 108 (debug). The earlier "5 observations, all one site" was
the PLAIN-shape number with the shape unstated.

**No verdict depends on the marking at either site.** Three reasons, each
verified this session:

1. `preserves.rs:381-383`'s `if has_call { ever_clobbered = [true; 16]; }` runs
   BEFORE the marking, and `has_call` measured TRUE at 100% of observations at
   both sites (both procs contain `jbsr`). Every bit is already set; the marking
   cannot flip one. This is the load-bearing reason.
2. `ever_clobbered` is consumed at exactly ONE place (`preserves.rs:447`, the
   `bailed_reached_exit && clobbered` arm), where a set bit can only turn a
   verdict INTO `Unverifiable` — a refusal, never a credit.
3. The "and anyway the callee does not clobber the checked register" argument
   does NOT hold in general: whether the oracle marks it is policy-dependent, and
   the instrumentation caught `CallPolicy::ClobberAll` passes marking all fifteen
   registers (including the checked `d7`/`a0`) at both sites. Reason (1) is what
   makes the marking inert, not the callee's declared clobbers.

So the widening (`Edge::TailOut | Edge::BranchOut`, identical body) is right
because OVER-obligation is that arm's safe direction; a `TailOut`-only widening
would have been a silent narrowing with no measurable effect today. Recorded so
the census is re-cited neither as "`BranchOut` has no corpus witness" nor as "a
false negative was averted".

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
| (not predicted) `BranchOut` in `preserves`'s tail-clobber marking — a CONSUMER-scoped count, not a corpus count | **2 sites** (`bne Player_DebugMove`, `bne TestPlayer_Debug`): 5 observations / 1 site in the plain s4, config_b and lean shapes; 14 / 2 sites in s4.debug and config_a; 0 in both demo shapes. Against 39 `TailOut` (plain) / 108 (debug). 28 `BranchOut` on the Z80 side in `z80_preserves`. Corpus-wide the figure is much larger: **20 68k conditional-branch sites whose target leaves the enclosing proc, across 9 procs** (s4.debug; 19 across 8 in plain) — re-derived this session by instrumenting `Cfg::edges`. Only the two sites inside `preserves`-declaring procs reach the marking; the gap is the consumer's own `ReturnScope::AllReturns` gate, not a property of `BranchOut`. |

## Bars

1. **Byte bar — BYTE-NEUTRAL, 7/7.** Target list derived from
   `crates/sigil-harness/golden/` (s4, s4.debug, demo, demo.debug, config_a,
   config_b, lean = seven). Full-file CRC / size, post-edit, identical to the
   committed chain-49 goldens and to the pre-edit seed-proof capture:
   `3b6cad91/411167`, `e3963874/423571`, `b8df1c2b/91330`, `30173928/94031`,
   `7660f157/423949`, `ace527ba/301205`, `69c20328/379110`. Anchors likewise
   unmoved. Zero bytes moved.
2. **Full strict — 3510 passed / 0 failed / 4 ignored across 312 binaries.**
   Closing arithmetic: 3510 + 4 = 3514 == this branch's own `#[test]` total
   (3514, counted this session). Master `06f52445`'s total, counted this session,
   is 3506; the +8 delta is EIGHT new test functions (an earlier draft of this
   packet described it as "eight new minus one rename" and mis-labelled which
   test the rename was — the diff shows `an_unroutable_split_is_refused` was
   renamed to `a_djnz_leg_out_of_the_body_is_refused`, same input and a new id,
   and the sweep is a wholly NEW function; the net is unaffected and the tree is
   better than the earlier description):
   `a_branch_out_always_has_a_sibling_and_a_tail_out_never_does`,
   `a_computed_transfer_is_a_singleton_tail_out`,
   `a_djnz_leg_that_leaves_the_body_keeps_its_edge`,
   `a_conditional_branch_out_is_not_a_required_return_path`,
   `the_same_target_reached_unconditionally_is_a_required_return_path`,
   `a_conditional_branch_out_does_not_excuse_the_returning_path`,
   `a_split_cost_conditional_is_refused_before_its_edges_are_counted`, and
   `a_split_cost_terminator_presents_exactly_two_edges` (added in the fixup
   round; the +1 over the pre-fixup 3513 is exactly this function, chased by
   diffing the `#[test]` name sets of `e19d633c` and the working tree — the sweep
   RENAME in the same round is count-neutral).
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
- Deleting the now-inputless `AmbiguousBranch` VARIANT — kept as a defensive
  guard; ledgered AND documented in code with its kill condition. The lint ID is
  NOT dead and was never a deletion candidate: `z80_cycles`'s span-builtin
  producer is live and pinned.
- Charging `out_verify`'s `BranchOut` skip — the hole is real and in the module's
  dangerous polarity, but closing it needs a divergence marker readable at a
  conditional target. Retexted as a known-incompleteness pin and ledgered OPEN.
- Pinning the `djnz`-to-`@noreturn` widening (below) — ledgered with its
  soundness argument; a unit pin is offered to the next `cycle_budget` parcel.

## Seed proof

The aeon worktree was seeded with six gitignored paths (the boilerplate lists
two). Proven BEFORE any edit: all seven targets built byte-identical to the
committed goldens. Evidence on the four extra seeds: every file in
`engine/debug/generated/` and `engine/sound/generated/` had its mtime advanced to
build time (they are build OUTPUTS), and `build.sh:113-117` auto-builds
`tools/salvador` → `tools/bin/salvador` when missing. So the boilerplate's
two-item list appears sufficient and only `games/sonic4/data/editor/` is
genuinely required — evidence, not proof, since a bare worktree was not tested.

## Fixup round (lens panel A/B/C, 2026-08-06)

The code was found SOUND: Lens C traced every consumer end-to-end with no
MUST-FIX, and Lens B independently confirmed byte-neutrality, the strict
arithmetic and every census. What the panel refuted was DOC TRUTH, in two claims
this packet had written into the permanent record. Both are restated above.

1. **The `BranchOut` witness was wrong twice over** — one site became two
   (`test_player.emp:129 bne TestPlayer_Debug`, inside `TestPlayer_Main`), and
   the "a `TailOut`-only widening would have been a `preserves` FALSE NEGATIVE on
   a shipping path" claim is UNDEMONSTRATED and is withdrawn. Measured: the
   marking is inert at BOTH sites because `has_call` is true at 100% of
   observations, and `ever_clobbered` can only push a verdict toward
   `Unverifiable` anyway. **No false negative is demonstrable at either site.**
   The widening is right on POLARITY grounds.
2. **The census framing was a consumer count read as a corpus count.** Corpus-wide:
   20 68k conditional-branch sites leaving their proc across 9 procs (s4.debug),
   19 across 8 (plain). The census table row is now scoped to its consumer.
3. **`[cycles.ambiguous-branch]` is a live lint id**, and only the
   `BudgetFindingKind` VARIANT is inputless — for two different reasons, with the
   `call cc` ordering dependency now demonstrated by revert and pinned.
4. **The sweep did not measure what it was named.** All ten shapes are refused by
   the STRUCTURAL loop before `two_way` is computed; verified by hand that
   dropping the `djnz` fall-through push leaves the sweep green. Retitled, and
   the edge-count invariant pinned crate-side where it can actually be observed.
5. **One unreported weakening**, ledgered: `djnz Target` where `Target` is
   `@noreturn` turned a refusal (`AmbiguousBranch`, from the dropped leg leaving
   one edge) into a CHARGED budget that can pass. Sound (the leg diverges, taken
   cost charged with `succ: None`) and corpus-inert, but it belonged in the
   record.
6. Doc/style: a comment severed mid-sentence in `cycle_budget.rs` restored (it
   had lost the `|| names_a_target` clause and drifted from its untouched twin);
   `ExitKind`'s enum doc no longer claims a 1:1 `Edge` correspondence this commit
   broke; the `z80_preserves` `Defer` cross-reference and the append-after-the-
   close comment fixed; `OutFlavor::{Tail, Branch}` renamed to
   `{TailOut, BranchOut}` so all four call sites read as the edge they produce;
   `flag_check`'s "unconstructible" claim narrowed to the route that was actually
   checked (a conditional lowering to `SymOff`/`AbsSym` reaches that arm today
   and is unaudited, not proven unreachable); the `dbra` probe spelled in its
   real two-operand form; five ragged comment blocks re-wrapped; the stray EOF
   blank line dropped.

Byte bar re-run after every edit: **7/7 unchanged**, same CRCs as above. Warn
tiers: identical lint-id set on all seven shapes.
