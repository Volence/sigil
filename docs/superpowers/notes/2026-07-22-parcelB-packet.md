# Parcel B — D1c contract-precision (byte-neutral) — build packet + firing snapshot

Branch `pass3-parcelB-hoists` off aeon master `923ba5d` (worktree `aeon-pb`). 2 commits:
`b1bc4b8` (B1 Load_Object d2) → `fd452e8` (B2 caller tightenings). **Byte-neutral** —
declaration/comment only, no eval/lower/backend touched.

## What shipped

4 over-declared-clobber tightenings (the "honest optional tidy" the G4 Stage-A
checkpoint flagged), each tightening a proc's declared clobbers to its verified
`effective` set; headers made accurate + lockstep in both twins:

| Proc | Was | Now | Freed reg | Why over-declared |
|---|---|---|---|---|
| `Load_Object` | `clobbers(d0-d3,a2,a3)` | `clobbers(d0-d1/d3,a2,a3)` | **d2** | read-only (subtype/flags source); `AllocDynamic` is `clobbers(d0)` |
| `EntityWindow_RescanObjects` | `clobbers(d0-d5,a0-a3)` | `clobbers(d0-d4,a0-a3)` | **d5** | reaches it only via `TrySpawnObject` (now `preserves(d3,d5)`) |
| `EntityWindow_ScanObjectsRight` | `clobbers(d0-d5,a0-a3)` | `clobbers(d0-d4,a0-a3)` | **d5** | same — transitive via `TrySpawnObject` |
| `EntityWindow_Scan` | `clobbers(d0-d7,a0-a5)` | `clobbers(d0-d7,a0-a4)` | **a5** | never in the effective set |

*(Attack-the-diff caught a bonus: the `Scan` a5-drop resolves a pre-existing inconsistency —
`EntityWindow_Init` `falls_into EntityWindow_Scan` already declared `a0-a4`, so the old `a5`
claim on `Scan` disagreed with its own fall-through parent. The header `a0-a3` widening on
RescanObjects/ScanObjectsRight verifies as accuracy: a1 enters transitively via `TrySpawnObject`
`clobbers(a0-a3)`, and the d5 freedom rests on `TrySpawnObject`'s §5-verified `preserves(d3,d5)`.)*

## Method: `declared ∖ effective`

For each proc, `ProcNode.declared_clobbers − Closure.effective[proc].regs` (the closure's
transitive effective set already subtracts §5-verified preserves); a non-empty, non-`out()`
remainder = an unexercised (over-declared) clobber. Ran whole-corpus; the D1c-derived set
gave exactly these 4. RescanY / RescanRings / ScanRingsRight were checked and write their
"freed" candidates themselves (effective includes them) → no tidy. Banked as the future
`[proc.clobber-unexercised]` lint's regression seed (gap-ledger row).

## The finding (roadmap item 6 reframe)

**The D1c-clear fuel contained NO byte-changing hoists.** D1c fires only when a caller reads
a register *after* a call *without* reloading it — so the sites are TIGHT BY CONSTRUCTION,
nothing to remove. And the freed registers are not hoistable loop-invariants: `Load_ObjectList`
reloads d2 fresh each iteration (`move.w (a0)+, d2` = per-object subtype); RescanY/Scan write
d5/d6 in their own bodies. So Parcel B is contract-precision (byte-neutral), not hoists — the
**second** time the net converted anticipated byte-surgery into contract precision (after
S2-D6 #3's stage-0). LICM hoist-hunt rejected by the overseer (no frame-lag pressure — ≥35%
idle every regime; Parcel D carries the EV-ranked review candidates).

## Firing snapshot — after `fd452e8`, ALL surfaces

| Surface | Count | Note |
|---|---|---|
| Closure `[proc.clobber-undeclared]` (transitive ERROR) | 0 | tightenings only shrink declared; effective unchanged |
| `[call.flag-result-unused]` §6 | 0 | — |
| `[call.input-undefined]` D1b (ERROR gate) | 0 | — |
| `[call.live-clobbered]` D1c | **unchanged** | `calls` test 29/0; the 2 documented FPs untouched |
| `[proc.out-unverified]` out-verify | **unchanged** | `out_verify_corpus` PASS |
| Dead-save worklist (D1d) | **0** | — |
| dropped instructions | 0 | — |

Strict `sigil-frontend-emp` (worktree, `SIGIL_STRICT_GATE`): **1553/0**. Byte gate:
both shapes reproduce canonical **748ca5ba** / **d5d8e163** exactly. No ripple, no
PROVENANCE re-baseline (byte-neutral).

## Findings by class

- **step-3 (correctness):** none — declaration precision only, machine-verified (effective set).
- **step-5 (optimization):** none byte-changing; the tightenings widen the caller-freedom pool
  (verified register survival) for future work, at zero ROM cost.
- **neither-bucket:** the `declared ∖ effective` sweep is a ready-made `[proc.clobber-unexercised]`
  lint (regression seed banked); Parcel D is where the review's actual EV-ranked code opts live.

## Gates
- Byte-neutral: both shapes 748ca5ba/d5d8e163 (belt-and-braces per the ruling).
- Strict frontend-emp 1553/0; out_verify_corpus + calls (D1c) tests green (counts unmoved).
- Attack-the-diff: overseer light review of the 4-tightening diff, pre-merge (granted).
