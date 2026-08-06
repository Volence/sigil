# The enumerated-dispatch form — `targets(...)` (2026-08-05)

Status: RULED (Fable). Closes the gap-ledger enumerated-dispatch row's design
half ("needs its own spelling and a soundness story"); the B′-3b packet §11
gap 2 and §12 step-3 are the demand documents. Scout ground truth 2026-08-05:
the budget walk's DAG min/max already handles arbitrary `Follow` fan-out; the
`Cfg` already owns `label_target`/`is_local_label`; the refusal point already
holds the operand vector.

## 1 · The spelling

A trailing clause on a computed-transfer instruction, naming the finite set of
labels the transfer can land on:

```
jmp     .jump_table(a1)     targets(.slot_0, .slot_1, .slot_2, .slot_3,
                                    .slot_4, .slot_5, .slot_6, .slot_7,
                                    .slot_8)
```

The enumeration names the PHYSICAL LANDING points — the labels sitting at the
table's stride offsets — never labels downstream of them. Naming a downstream
label (the drain a slot branches to, rather than the slot itself) walks the
budget PAST the slot's own dispatch code and produces a machine-checked
number the hardware exceeds; the dma_queue adoption's regression pin
(`targets_charge_the_landed_on_code_not_a_downstream_label`) exists because
this parcel made exactly that mistake first. Landing labels cost zero bytes —
place one at every `table + k·stride` offset and enumerate those.

- Legal ONLY on an unconditional computed transfer (a `jmp`-class mnemonic
  whose edge today is `Defer` with no resolvable `Sym` target: the
  `DispSymInd` / `PcRelIdx` / `IndIdx` / `Ind` operand shapes). On a direct
  `jmp .label` it is refused (`[dispatch.targets-redundant]` — the edge is
  already exact); on any call (`jsr`/`jbsr`/`bsr`) it is refused
  (`[dispatch.targets-on-call]` — calls are the opaque-call problem, which is
  callee-cost composition, deliberately NOT this form).
- Every named target must resolve to a LOCAL label of the enclosing proc body
  (`[dispatch.target-unknown]` / `[dispatch.target-nonlocal]`). Cross-proc
  enumerations are refused in v1: the census shows every cross-proc dispatch
  site also needs callee costs, so a nonlocal arm would buy nothing and the
  refusal keeps the form's meaning sharp.
- Duplicates refused (`[dispatch.target-duplicate]`); empty set refused (an
  enumeration that enumerates nothing is a contradiction in a mnemonic
  position).
- Composes with `as ContractType` (orthogonal: `as` bounds what an installed
  target may CLOBBER, `targets` bounds where control may GO); composes with
  authorship untouched.

## 2 · The soundness story (the ledger's demand — stated, not waved at)

**Exhaustiveness is the author's claim.** The compiler verifies existence,
locality, and distinctness of every named label — it does NOT verify that the
runtime index stays inside the table (that is the site's index-hygiene
discipline: the `andi` clamps and `ensure` geometry pins the corpus already
writes, e.g. dma_queue's `ensure(DMA_CRITICAL_SLOTS == 8)` +
`ensure(sizeof(DMAEntry) == 14)`).

Because the claim is author-asserted, its blast radius is confined to the one
analysis that is itself OPT-IN: **only the cycle-budget walk consumes
`targets(...)`** (a `Defer` with a target set becomes N `Follow` edges in
`charged_edges` — and ONLY there). The preserves prover, the flag walks, and
the clobbers closure keep treating the instruction as `Defer` — a wrong
enumeration can therefore mis-measure a budget the author explicitly asked to
have measured, and can corrupt nothing else. The day a soundness-bearing
analysis wants to see through a dispatch, the enumeration must come from a
structure the compiler OWNS (the typed jump-table decl of ledger row 1140 —
`Table : [ContractType; N]` — whose member list is exhaustive by
construction). This spec deliberately leaves that as the named successor, and
the packet must link the two rows.

Implementation note: the clause lives on `CodeItem::Instr` as carried data
(a `targets: Vec<String>` alongside `as_type`); `charged_edges` resolves the
names through the existing `label_target` map at walk time. No `Cfg` edge
construction changes — the conversion happens in the budget walk's edge
consumption, keeping every other Cfg consumer byte-for-byte on today's
behavior.

## 3 · What it charges

At the enumerated instruction the walk charges the instruction's own fixed
table cost (already in the B′-3b table for the `(d16,An)`/`(d8,An,Xn)`-class
`jmp` forms) once, then max/mins over the `Follow` set exactly as it does for
a two-edge branch today. Enumerated targets that form a cycle fall to the
existing `unbounded-loop` refusal naturally — no new rule.

## 4 · Adoption (this parcel)

**One site, the flagship**: `engine/system/dma_queue.emp Process_DMA_Critical`
gains `targets(...)` over its nine slots and the `@budget` its prose has
carried since before the cycle table existed. The porter measures what the
walk actually computes (the corroborated arithmetic says ceiling 670,
post-relaxation 662) and pins the declaration at the computed ceiling with the
prose numbers reconciled in the packet — if the walk's number is NOT ≤ 670,
STOP: either the table changed or the walk is wrong; do not adjust the prose.

**Explicit non-adoptions, recorded**: animate.emp `.cc_table-4` (arms carry
`jsr (a2)` → opaque-call), player_sensors ×2 (arms `jbsr` the probes),
player_common (cross-proc targets + jsr twin), s4lz ×2 (targets are unrolled
instruction addresses, not labels — not this form's shape, and the loop
refusal binds anyway). Each keeps its refusal; the packet lists them so the
non-adoption is a statement, not an omission.

## 5 · Bars

Byte-neutral parcel: `targets(...)` is carried data consumed at analysis time
— zero emitted bytes may change; seven-target byte bar identical. Full strict
with closing arithmetic; negative probes for every refusal named in §1; a
positive integration pin: dma_queue's `@budget` verifies, and a perturbation
pin (add a cycle to one drain slot in a fixture, the budget fires). Gap-ledger:
the enumerated-dispatch row CLOSED pointing here; row 1140 (typed jump-table)
gains the successor note. The B′-3b packet's "honestly refused" adoption
paragraph gains the pointer that the refusal is now lifted at the flagship.
