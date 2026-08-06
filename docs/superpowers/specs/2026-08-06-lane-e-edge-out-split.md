# Lane E — the Edge out-split: `Defer` → `TailOut` / `BranchOut` (2026-08-06)

Status: RULED (Fable). Closes ledger row 2158 (the `Defer` successor split the
cfg-edges parcel deliberately declined). Fourth instance of the mnemonic-table
defect family; the fix deletes the fourth table.

Scout ground truth, re-verified in the tree 2026-08-06 (the commissioning
handoff's row citations had drifted — everything below was read fresh):

- `flag_check.rs:613` `enum Edge` = `Follow` / `Return` / `FallOff` / `Defer`;
  the `Defer` doc (:625-628) already admits it covers two facts ("a tail
  transfer, **or** a conditional branch whose target is external").
- The `Defer` decision lives in the shared three-way `branch_edge`
  (flag_check.rs:440-449), which has TWO `Defer` arms — external symbol (:445)
  and no-symbol/computed target (:447) — and FOUR call sites: the 68k
  unconditional arm (:321-326), the 68k conditional taken edge (:343), the Z80
  unconditional `jp`/`jr` (:388-390), and the Z80 conditional `jp cc`/`jr cc`
  (:399-405). The conditional/unconditional axis is known only at the call
  site, never inside `branch_edge`.
- The `djnz` arm (:407-418) does NOT route through `branch_edge`: a taken leg
  that misses `label_target` (external target, or a trailing local) is
  silently DROPPED — no edge at all, a missing-path hole, ledgered as inert.
- The consumer that re-derives the axis: `out_verify.rs:144 is_uncond_tail`
  matches `bra|jbra|jmp|jra` — a private duplicate of `flag_check.rs:62
  UNCOND_MNEMONICS`, identical today, unsynchronized by construction; consumed
  at the `Edge::Defer` arm (out_verify.rs:332-355). Drift polarity: false
  where the builder meant a tail ⇒ the required-return path is skipped ⇒
  FALSE NEGATIVE on `[proc.out-unverified]`.
- Stale doc found during the audit: the `Edge` doc (flag_check.rs:601-602)
  names "`z80_preserves`'s own Z80 builder" as a third edge builder. No such
  builder exists — `z80_preserves` consumes `Cfg::z80_edges`
  (z80_preserves.rs:9, :409). Fix rides this parcel.
- `out_verify`'s corpus pass skips Z80 modules by design
  (corpus_contracts.rs:380) — the 68k-only consumer claim holds.
- `z80_bus.rs` consumes NO `Edge` at all (the commissioning note listed it
  among the consumers to audit; it is a non-consumer — grep is empty).

## 1 · The split — semantics, and the no-target-claim rule

Two variants replace `Defer`; `Defer` itself is DELETED, no variant left
behind, so the compiler forces every consumer arm to choose:

- **`Edge::TailOut`** — the successor of an UNCONDITIONAL transfer that
  leaves the body: an external symbol (`jbra Foo`, `jp Foo`), a symbol-offset
  or absolute form the abs seam lowers, or a computed target (`jmp (a0)`,
  `jp (hl)`). Control leaves the proc on every execution that reaches the
  instruction; a `TailOut` is the instruction's ONLY edge.
- **`Edge::BranchOut`** — the TAKEN successor of a CONDITIONAL terminator
  whose target is outside the body. Always accompanied by a sibling local
  edge (`Follow` or `FallOff`); never an instruction's only edge.

**Neither variant carries any claim about the target** — not whether it
returns, diverges, or falls onward. That is row 2158's bounding argument,
inherited here so no consumer relitigates it: `Return`/`FallOff` are facts
about the MACHINE; what the target does is not builder-visible and must not
be guessed. The conditional/unconditional axis of the TERMINATOR is
builder-visible, IS a property of the instruction, and is the entire content
of the split — not one inch more. No `TailCall`/`DivergesOut` flavors, ever,
in this enum.

## 2 · Builder mechanics

`branch_edge` cannot decide the axis (it sees only operands); the flavor is
passed in from the call site — a parameter (or two thin wrappers), mapping
both of its `Defer` arms (:445, :447) to the caller's flavor. The
`Follow`/`FallOff` arms of the three-way are untouched.

- The computed arm (:447) inherits the caller's flavor: a computed `jmp
  (a0)`/`jp (hl)` is a `TailOut` — which matches `is_uncond_tail`'s current
  mnemonic view (it already treats a computed `jmp` as a tail), so no
  consumer sees a new fact. A CONDITIONAL computed target is unconstructible
  on both ISAs (68k `bXX`/`dbXX` take a label; Z80 `jp cc` takes `nn` only);
  map that arm to `BranchOut` anyway with a comment saying so — do not
  assert-unreachable on operand shapes the parser may later extend.
- **`djnz` (commissioning Q6, ruled here — this IS the "if ever unified"
  moment):** route the `djnz` taken leg through the shared `branch_edge` with
  the `BranchOut` flavor, deleting the raw `label_target` lookup that
  silently drops an unresolvable leg. Today an external or trailing-local
  `djnz` target loses its edge entirely — a missing-path hole (walks never
  see the leg). After: external target → `BranchOut` + fall-through;
  trailing-local → `FallOff` + fall-through. Corpus census: every corpus
  `djnz` targets a backward in-body local label — zero external, zero
  trailing — so this is a zero-corpus-effect soundness fix, landed on unit
  pins (§7).

## 3 · CPU uniformity (commissioning Q3): UNIFORM

Ruled on soundness grounds, not symmetry:

1. The enum is shared. A 68k-shaped split would leave `Defer` alive for Z80,
   and every shared consumer keeps a third arm forever — the exact breeding
   ground this family grows in.
2. Both CPUs already route through the shared `branch_edge`; the flavor
   costs one parameter at call sites that already know the answer.
3. Z80 has a live corpus `TailOut` (sound_psg.emp:252 `jr VolEnv_ResolveScan`
   inside `FmVolEnv_Resolve`, an `out(hl, carry)` proc), charged today by
   `z80_preserves`' tail-callee oracle.
4. No Z80 consumer distinguishes the flavors TODAY (`out_verify` is 68k-only,
   corpus_contracts.rs:380) — the uniformity protects the builder invariant
   ("the builder decides, once") so the NEXT `is_uncond_tail` cannot be born
   on the Z80 side.

## 4 · Blast radius — audited per consumer

ZERO consumers change verdicts. Exactly ONE changes its derivation
(out_verify). The rest are mechanical arm-widenings with identical bodies.
The porter states each expected non-change, then MEASURES (strict counts +
seven-target bytes) — the audit below says which need no second look:

1. **out_verify.rs:332-355 — the beneficiary.** `TailOut` → the
   required-return arm with tail credit (`direct_target` STAYS for the credit
   lookup — that is a target-identity question, not an edge-flavor question);
   `BranchOut` → skip, unchanged polarity ("not a local counterexample,
   mirroring preserves"). DELETE `is_uncond_tail` (:142-146). Equivalence by
   case: computed `jmp` — mnemonic-view true = `TailOut`, same arm; external
   `bXX` — mnemonic-view false = `BranchOut`, same skip.
2. **preserves.rs — provably no behavior change.** :393's `any Defer` becomes
   `any (TailOut | BranchOut)`; the dataflow's map to `ExitKind::Defer`
   (:579) covers both variants. **`ExitKind` is NOT split** — both its
   consumers (entry-value proof, stack balance) charge the callee oracle
   identically for both flavors; splitting it would be surface with no
   consumer. Docs at :257, :490, :1301-1302 retext.
3. **z80_preserves.rs:486 — provably no behavior change.** The one arm gains
   the second variant, identical body (both flavors are exits charged
   through the oracle; over-obligation is its safe polarity). Doc :453
   retext.
4. **cycle_budget.rs — no behavior change, one equivalence to pin.** The
   singleton patterns `[Edge::Defer]` (:556, :635) become `[Edge::TailOut]`
   — equivalent because a `BranchOut` is never an instruction's only edge
   (§1); PIN that as a unit test rather than asserting it in the builder.
   The `divergent_terminal` arms (:729, :775) and the refusal/None arms
   (:735, :742, :776, :821) widen to both variants, identical bodies.
   The test pin at :1472-1475 (`vec![Edge::Defer]` for the computed
   dispatch) updates to `TailOut`. Confirm `Process_DMA_Critical`'s
   `@budget 670` is unmoved.
5. **context.rs:367 — no behavior change.** `Return | FallOff | Defer` gains
   both variants, same escape arm.
6. **flag_check.rs:886 `abandons_flag` — no behavior change.** "Flows out"
   is true of the taken edge in both flavors; the conditional's sibling
   `Follow` edge is walked separately, exactly as today.
7. **Builder tests** :1066, :1102, :1106 update expectations
   (`[Defer, FallOff]` → `[BranchOut, FallOff]`, etc.).

Non-consumers, audited so nobody audits them twice: **z80_bus.rs** (zero
`Edge` usage — the commissioning list was wrong to include it),
closure.rs:160 (prose only), value.rs:435 + the cycle_budget module doc
(retext only), calls.rs / type_slice.rs / corpus_contracts.rs (no `Edge`
consumption; corpus_contracts orchestrates but never matches an edge).

## 5 · Scope rulings

- **The walk-level `falls_into` policy field (Q4): OUT of lane E.** E's bar
  is verdict-neutral; the policy field exists to CHANGE obligations for the
  three consumers the 2026-08-05 edge-split row names — a behavior-changing
  successor with its own witnesses and adjudication. It is also an
  orthogonal axis (`FallOff` policy vs `Defer` split). That row stays OPEN,
  unabsorbed.
- **The ISA-crate `is_call`/`is_return`/`is_branch` classifier (Q5): LEFT
  STANDING.** E deletes one duplicate table and shrinks that row's evidence
  base by one instance; it absorbs nothing — the demand is cross-crate and
  wider than any one analysis. Note the deletion on that row when closing
  2158.

## 6 · Corpus witness census (Q7) — demanded up front, delivered up front

Lane E is NOT a unit-pins-only parcel (unlike t-edges, which landed with
zero corpus positives). Live witnesses, scanned 2026-08-06:

- **68k `TailOut`, credit path:** `Art_Decompress` (`out(a0, a1)`,
  engine/level/load_art.emp:46) ends `jbra ZX0_Decompress` — itself
  `out(a0, a1)` (zx0.emp:58). The tail-credit arm executes on the real
  corpus and must stay green.
- **68k `TailOut`, no-credit flavor:** `QueueDMA_Critical` and
  `QueueDMA_Important` (dma_queue.emp) end `jbra QueueDMA_Deferrable.transfer`
  — a SymOff target, so `direct_target` yields no credit and the outs must
  be locally produced (they are today; must stay green).
- **Z80 `TailOut`:** sound_psg.emp:252 `jr VolEnv_ResolveScan` (§3).
- **`BranchOut` inside an `out()` proc: NONE found** — the out_verify
  skip-arm lands on unit pins only. A prediction, not a discovery.
- **`djnz` external/trailing: ZERO corpus sites** — the §2 ruling lands on
  unit pins only. Same status: predicted here.

The scan is line-based (grep/awk over proc blocks); the porter re-confirms
at branch time via the strict run's invariance plus unit fixtures mirroring
each witness shape above.

## 7 · Bars — hit them or stop

- **Byte bar:** byte-NEUTRAL across all seven golden targets at the chain
  current at branch time (49 at commissioning). One moved byte = STOP and
  report; this parcel has no license to refreeze.
- **Verdict bar:** strict diagnostic counts identical pre/post — measured,
  not assumed. Zero new firings, zero lost firings.
- **Pins (all four):** (a) `BranchOut` is never an instruction's only edge;
  (b) external `djnz` → `[BranchOut, Follow]`, trailing-local `djnz` →
  `[FallOff, Follow]`-shaped lists per §2; (c) computed `jmp` → singleton
  `[TailOut]` (update cycle_budget.rs:1472-1475); (d) out_verify unit cases
  for the `TailOut` credit read and the `BranchOut` skip.
- **Doc bar:** fix the stale third-builder claim (flag_check.rs:601-602);
  extend the `Edge` doc with the §1 no-target-claim sentence; all comment
  edits present-tense contract facts, no change-history narration.
- **Panel:** standard A/B/C lens panel before merge (era bar).

Stop conditions beyond the bars: any corpus diagnostic changes; a FIFTH
mnemonic-table site discovered mid-parcel (report it as a finding, do not
absorb it into E).
