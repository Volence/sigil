# The `[proc.out-unverified]` residue, triaged over the fixpoint

Measured on merged sigil master `3e0824b1` against aeon `d8c93d7`, by
perturbation rather than by reading the baseline array. 30 rows, and they are
**not** 30 independent sites — but they are also not one cause.

## Method

Two perturbations, each reverted by string-replace, each measured as a set diff
against the unperturbed residue (never as a count):

1. **Declared-vs-verified credit.** Point the residue surface at the DECLARED out
   maps instead of the VERIFIED ones. A row that VANISHES has no local production
   gap at all — its only problem is upstream (a dependant). A row that SURVIVES
   has a genuine local gap (a root).
2. **Width rule off.** Make every data-register write count as a full-width
   production. A row that VANISHES is a WIDTH gap — the body does produce the
   value, just not in all 32 bits. A row that SURVIVES is a genuinely unproduced
   path, and no width facet on `out` will help it.

## Result: 30 rows, four causes

| cause | rows | closes via |
|---|---|---|
| WIDTH gap (sub-width production) | **15** | `out(dN: u8/u16)` — the standing ruling |
| `probe_core` d1/d2 unproduced | **8** | the `.cl_hanging` trace (collision step 2) |
| the `S4LZ_Decompress::a1` chain | **3** | one fix at the root |
| `DrawRings` / `InsertSpriteMasks` | **4** | unexamined; two procs, same shape |

### The cascade is REAL but NARROW — correcting this lane's own emphasis

Only **2 of 30** rows are pure dependants: `Art_Decompress :: a1` and
`S4LZ_DecompressDict :: a1`, both grounding in `S4LZ_Decompress :: a1`. The
remaining 28 have a local production gap.

The earlier ledger row warned that "a burn-down that counts rows will over-count
the work". That is true and worth keeping, but the measured magnitude is small,
and the row's framing implied a larger effect than exists. **The census's real
value turned out to be the OTHER axis** — what kind of local gap the 28 have —
not the cascade split. Recorded because the correction is the point: the reason
to measure was never that the answer would be dramatic.

`S4LZ_Decompress :: a1` is still the single highest-leverage row: it holds THREE
open (itself plus both dependants), and closing it also ARMS the dormant site-2
`falls_into` plumbing guard. **When it closes, re-run the site-2 mutant and
confirm it goes RED. If it stays green, that is a finding, not a footnote.**

## The width gap is HALF the residue, not the dominant cause

Standing premise (brief, and `contract_baseline.rs`'s own header): the residue's
"dominant" cause is the language-surface width gap. **Measured: exactly 15 of 30
— half.** The other 15 are genuinely unproduced paths that `out(dN: type)` cannot
touch.

The 15 width-gap rows, which ARE the adoption target:

```
Collision_GetType::d0   Collision_Probe{Down,Left,Right,Up}::d0
Emit_ObjectPieces::d5   EntityWindow_DeriveWindow::{d2,d3,d4,d5}
EntityWindow_EntryForSection::d0   GetSineCosine::{d0,d1}
Section_RedrawPlanes::d7   Tile_Cache_GetTile::d2
```

Address-register rows can never be width gaps — 68k address writes are full-width
by the rule — so the 5 `a1`/`a4` rows were excluded by construction, not by
measurement, and the check agrees.

## `probe_core` — the census independently corroborates collision step 2

The four `Collision_Probe*` procs are one macro body (`probe_core`,
`player_sensors.emp:~170-214`) stamped four times, which is why they fire
identically. The split within them is the interesting part:

- **`d0` is a width gap** in all four — the partial-height return reaches `rts`
  with `ext.w d0`, a `.w` write.
- **`d1`/`d2` are NOT** — they survive the width-off probe, so they are genuinely
  unproduced on some path.

Reading the body, `d2` is written in exactly ONE place: `.cl_air`'s
`moveq #0, d2`. Neither `.cl_hanging` (which ends `moveq #16, d0` / `rts`) nor
the partial-height `rts` writes `d2` at all.

**So the census arrived at collision step 2's question from the opposite
direction, and sharpens it.** Step 2 was framed as "trace `.cl_hanging` →
`.full_back` in the oracle and decide real bug vs benign-by-downstream-filter vs
contract-only". The census says the register to put under the lens is **`d2`,
unwritten on two of the return paths**, and that whatever the verdict is, it
accounts for 8 of the 15 non-width rows — the largest single non-width cause.

One caveat kept honest: `d1`'s failure is not yet explained. It is written `.b`
at the attr/angle read, which the width-off probe should have credited, so a path
must reach a return without passing that write. That path was not identified
here and is a live question for step 2, not a settled fact.

## What this changes about the queue

Nothing about the ORDER, which stands. What it changes is the expected yield:

- Item 2 (`out(dN: type)`) closes **15 of 30**, not "most". Still the largest
  single lever, and the per-site caller read-width sweep applies to 13 procs.
- Item 3 (`.cl_hanging`) is worth **8 rows** and has a named target register.
- `DrawRings` / `InsertSpriteMasks` (4 rows, `a4` + `d5` each) are an unexamined
  pair sharing a shape — a cursor and a counter unproduced on some early exit.
  Cheap to look at, and nothing depends on them.

Together items 2 and 3 account for 23 of 30; the S4LZ root takes it to 26.
