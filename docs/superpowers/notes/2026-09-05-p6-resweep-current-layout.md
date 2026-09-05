# P6 re-swept at the current layout — the counts could not have moved, and the margin did

**Date:** 2026-09-05
**Subject:** row **P6** of `2026-09-05-decouple-aeon-side-inventory.md`, and §6 point 3 / item (b)
**Instrument:** `scripts/p6_resweep.py` (this repo), driving aeon `tools/dplc_straddle.py`
**Aeon revision measured:** `305af22217b4a8fbf055eaa301bd484aba7c133c` (aeon `origin/master` at dispatch)
**Verdict in one line:** the "not yet" **stands**, but almost none of the note's stated
reasoning for it survives contact with the measurement.

---

## 0. The headline

| figure | P6 (`15cb42f7`, 2026-08-29) | today (`305af222`, 2026-09-05) | moved |
|---|---|---|---|
| `Art_Sonic` failing positions | 2,773 of 131,073 | **2,773 of 131,073** | **unchanged — and structurally could not change** |
| forbidden bands | 43 | **43** | unchanged |
| band widths min / median / max | 31 / 31 / 415 B | **31 / 31 / 415 B** | unchanged |
| worst peak SLOTS | 17 | **17** | unchanged |
| margin, *shrink* direction | 5,188 B | **8,290 B** | **+3,102 B — better** |
| margin, *growth* direction | 36,092 B | **32,990 B** | **−3,102 B — worse** |
| `Art_Tails` / `Art_TailsAppendage` / `Art_Knuckles` | never measured | **0 failing positions each, over the full ±64 KB** | new |
| `BLOCK-STREAM-DEDUP`'s slack in its tightening direction | 5,686 B | **2,584 B** | **−3,102 B, a 55 % loss in seven days** |

Positions unmeasurable: **0**. Every swept position is arithmetic over a DPLC table that
loaded; a load failure raises `Unmeasurable` and the runner reports no numbers at all. The
`--gate` on this tree is **green** (worst peak SLOT cost 10 against a bar of 10, worst
reachable split 0 against a reserve of 2, concurrent demand 0 of 2).

---

## 1. The first finding is that P6's counts are not a measurement of a layout

The straddle predicate is `dplc_straddle.straddles`: an entry crosses when
`(src % B) + len > B`, with `src = art_base + tile_offset + d` and `B = 0x20000` derived from
`dma_queue.emp`'s split code. **The predicate depends on the base only through
`(base + offset + d) mod 0x20000`.** So for a fixed DPLC table the set of failing shifts `d`
is periodic with period `0x20000 = 131,072`, and moving the base by Δ does nothing but
translate that set by −Δ.

P6's sweep window is ±64 KB, which is **131,073 positions — exactly one period plus one**.
It is therefore not a sample of the neighbourhood; it is a **complete census of every residue
class**, with the two endpoints being the same residue counted twice.

That has a consequence the note does not draw, and it is the whole shape of this result:

> The failing count, the band count, the band widths and the worst peak are **invariant under
> any layout move whatsoever.** They are properties of `DPLC_Sonic`'s frame geometry modulo
> 128 KB. Only the **phase** — where shift 0 sits inside a fixed pattern — is layout-dependent.

Measured confirmations rather than assertions: the 2,773 failing positions occupy **2,773
distinct residues mod `0x20000`** (no double count), and **neither endpoint of the window is
failing**, so no band is cut in half by the window edge — the one circumstance under which the
band *count* could have differed between two phases.

**So the note's §6 point 3 asks for the wrong thing.** *"That measurement was taken at a
different layout"* is true and irrelevant to the four numbers the note quotes. The number that
was genuinely stale is the one the note quotes almost in passing: **the margin.**

### 1.1 The counts and mine are of the same object — proven, not assumed

Identical statistics from an independent run invite the suspicion that something reproduced
P6 rather than re-measured. It did not, and the reason is checkable: P6's sweep was run
*"holding the re-cut DPLC fixed"*, i.e. over a **modelled** re-cut of the then-shipping blob,
which `aa872628` then landed. Feeding `15cb42f7`'s blobs through `dplc_straddle.recut` and
comparing the result to today's shipping table:

```
15cb42f7 dplc 2368 B  art 97472 B   (peak entries 13)
305af222 dplc 2244 B  art 101056 B  (peak entries 10)
modelled re-cut: 6 frames rewritten, art +3584 B, dplc -124 B, peak entries 10
MODELLED == TODAY'S SHIPPING FRAMES: True
```

The two sweeps are over the **identical frame set**. The invariance above then makes the
identical counts a necessity, not a coincidence — and makes any *difference* in them the thing
that would have needed explaining.

---

## 2. What actually moved: the phase, by +3,102 B

The failing-shift set moved by −3,102, so **`Art_Sonic`'s base moved 3,102 B LATER** since
P6. Today `Art_Sonic` sits at `0x73942 + 101,056 B = 0x8C402`; P6's base is therefore
`0x72D24` (derived from the margin identity below, not measured from a build at that
revision — stated as a derivation).

That single delta was fixed from **one** of P6's numbers and then used to predict the others,
which is the only form in which this reading is falsifiable:

| prediction from Δ = +3,102, fixed by P6's *earlier* margin alone | measured today | |
|---|---|---|
| first failing *later* shift at +32,991 | +32,991 | **MATCH** |
| P6's combined dedup+re-cut safe band `[−29,796, −15,300]` appears at `[−32,898, −18,402]` | maximal safe run containing −32,898 is exactly `(−32,898, −18,402)` | **MATCH** |

Both hold. And the round trip settles a trap worth writing down: **P6's margins are stated in
the *last safe* convention** without saying so. Under the *first failing* convention the later
margin misses by one, which is why P6's `5,188 + 36,092` and my flanking-gap of 41,281 differ
by 2. They do not disagree; they count differently. The runner now prints both forms.

### 2.1 The mechanism is not the one the dispatch and the note name

The brief and the note both attribute the staleness to the **2026-09-04 re-layout moving the
sound banks by 96 KB**. That cannot be what moved `Art_Sonic`, and the tree says so plainly:

* The bank anchors are at `0xA8000` / `0xB8000`. `Art_Sonic` **ends at `0x8C402`**, entirely
  below both. Moving something above a symbol does not move the symbol.
* `map.toml` derives the anchors *from* the data: `dac_banks = align_up(packed_data_end +
  DATA_GROWTH_RESERVE + grace)`. The banks moved **because** the data grew. The causality is
  the reverse of the one the note assumes.
* `Art_Sonic` is the **highest symbol below the anchor** in the listing (`AngleTable`,
  `SolidityTable`, `CrossoverTable`, `Map_Sonic`, `DPLC_Sonic`, `Art_Sonic`), i.e. it is the
  terminus of the packed data region — B7's assumption, confirmed here incidentally.

So the phase moved for the dullest possible reason: **seven days of ordinary data growth
upstream of `Art_Sonic`** — +3,226 B of it, against the re-cut's own −124 B of `DPLC_Sonic`
shrink (which sits immediately before the art and pulls it *earlier*), netting +3,102 B. That
is about **440 B/day of drift in the base P6's whole result is a function of**, and nothing
watches it.

---

## 3. Extending to all four subjects: a null result, and it is a real one

The subject list was **derived, not taken from the brief**: `dplc_straddle.SUBJECTS`,
cross-checked at run time by the tool's own `subject_bindings()` against the three
`CharacterDef` literals and `tails_appendage.emp`'s `equ ... = extern(...)` block. It is:

**`Art_Sonic`, `Art_Tails`, `Art_TailsAppendage`, `Art_Knuckles`** — four.

Swept over the same ±64 KB:

| subject | base | frames (reachable) | peak entries | VERDICT A fails | VERDICT B fails |
|---|---|---|---|---|---|
| `Art_Sonic` | `0x73942` | 224 (83) | 10 | **2,773** in 43 bands | **31** in 1 band |
| `Art_Tails` | `0x2DD42` | 251 (76) | 2 | 0 | 0 |
| `Art_TailsAppendage` | `0x4A776` | 45 (36) | 1 | 0 | 0 |
| `Art_Knuckles` | `0x4F13E` | 251 (113) | 5 | 0 | 0 |

The zeros are **structural, not lucky**. A frame's slot cost is at most twice its entry count,
so the most a subject can ever cost is `2 × peak entries`: **4 for Tails, 2 for the appendage,
10 for Knuckles** — against a bar of 10. **Three of the four subjects cannot fail VERDICT A at
any base in the address space.** No sweep was needed to know it and the sweep confirms it.

Two things follow.

* **§6 item (b) is discharged with less risk than it priced.** *"Until that number exists for
  `0xA8000`, 'the packer may float' is unpriced"* — it exists now, and extending from one
  subject to four adds **zero** forbidden bands. The constraint surface is one subject wide.
* **Knuckles is one frame from joining Sonic.** Its ceiling is `2 × 5 = 10`, which is *equal*
  to the bar and so not over it. A single 6-entry frame added to `DPLC_Knuckles` would make
  Knuckles able to fail VERDICT A at some base, and nothing in the tree states that as a
  constraint. This is an F/G-class assumption of exactly the kind §5 enumerates, found in a
  new place.

### 3.1 One thing P6 could not have found

`dplc_straddle`'s **VERDICT B** (a *reachable* frame splitting past `DPLC_ENTRY_RESERVE`) did
not exist when P6's number was taken. Swept, it fails at **31 positions, one 31 B band at
`[+45,567, +45,597]`**, where a reachable Sonic frame splits **7** ways against a 2-slot
reserve. That is categorically worse than a VERDICT A breach: `.split_reject` drops the whole
transfer, so it is **a displayable frame whose art would not load**, not a budget overrun. It
is 45,567 B away in the growth direction and no live parcel points at it — recorded so that
"the bands are all budget breaches" is never assumed.

---

## 4. How free is the packer, really — and does this weaken the "not yet"?

**It weakens the note's reasoning in two of three places and strengthens the hazard in the
third, and the hazard is the one nobody was watching.** Plainly, in the order the note would
want it:

The packer is **not free, and its unfreedom is permanent rather than incidental**: 2.1 % of
one-byte positions are forbidden for `Art_Sonic`, that figure is a property of the DPLC table
modulo 128 KB, and no re-layout can improve it — which is a *stronger* statement than the note
made, because the note offered it as a contingent measurement someone might re-take and find
better. On the other hand the *scope* of the unfreedom is narrower than §6 item (b) feared:
three of the four straddle subjects cannot breach the bar at any address, so the sweep the
note wanted extended returns nothing new, and the single measured band the extension did find
is 45 KB away. Both of those **weaken the "not yet"** — the enumeration was carrying more
priced risk here than exists. What moves the other way is the phase. In the direction a shrink
pushes, the margin improved (5,188 → 8,290 B); in the direction ordinary growth pushes it
decayed (36,092 → 32,990 B); and the number that actually matters decayed hardest:
**`BLOCK-STREAM-DEDUP`'s slack above its safe band fell from 5,686 B to 2,584 B in seven
days**, because `Art_Sonic` is the terminus of the packed region and every byte added anywhere
below it moves the base. At the observed ~440 B/day that parcel's approved −20,986 B shift
walks into a forbidden band in **under a week of unrelated data growth**, with no gate that
would say so before the build turns red. So: the verdict survives, the argument for it does
not, and the live risk is a decaying margin rather than a stale census.

---

## 5. Method, and what would refute it

**The runner reimplements nothing.** `scripts/p6_resweep.py` imports aeon's own
`tools/dplc_straddle.py` from the tree under test, so `TILE_SIZE`, `DMA_IMPORTANT_SLOTS`,
`DPLC_ENTRY_RESERVE`, the `0x20000` boundary and the ratchet are all still derived from source
by the tool. FAIL means the tool's **VERDICT A** — `peak SLOTS > DMA_IMPORTANT_SLOTS −
DPLC_ENTRY_RESERVE` = 10 — because that is the predicate P6's number was taken under.

**The one addition is a fast path, and it is controlled.** 131,073 shifts × 4 subjects through
`frame_costs` is minutes of interpreter, so the predicate is vectorized over the shift range.
A rewritten predicate is exactly the shape that silently answers a slightly different question,
so `--control` (on by default) re-runs **512 shifts — both endpoints, zero, every band edge,
and a seeded random sample — through `dplc_straddle.frame_costs` itself** and requires exact
agreement on peak entries, peak slots and reachable split. One disagreement is fatal and the
run prints no numbers. All 512 agreed for all four subjects. Total wall clock: **1.0 s.**

**Provenance of the tree.** Provisioned with `scripts/provision-aeon-ref.sh` into a detached
worktree at `305af222`, never aeon's live checkout. Both `s4` shapes were built (plain and
`DEBUG=1`); the measurement uses `s4.debug.lst` + `s4.debug.bin`. ROMs: `s4.bin`
`8a47c755/819807`, `s4.debug.bin` `8bb835d7/845944`. Assembler: `sigil 0.1.0 (71bd2c19)`,
built from this worktree at this parcel's own first commit into a private `CARGO_TARGET_DIR`
— the shared `target/release/sigil` was never relinked.

**The freshness witness is not `repin --check`.** That witness is only valid at the revision
the goldens were frozen against, and this is deliberately a *different* revision, so the
provisioner correctly reports `golden control : not-applicable` and a pin mismatch there would
mean nothing. The witness used instead is a **content witness in the built listing**:
`Dac_Temp_Blip` resolves to **`0xA8000`**, the anchor address that only exists after the
2026-09-04 re-layout (`446a27d9`); before it the anchor was `0x90000`. A listing carrying
`0xA8000` cannot have been built from a pre-re-layout tree. Supporting: the worktree is at
`305af222` with a clean `status --porcelain`, and both listings post-date the build start.

**What would refute this note.** Any of: `DPLC_Sonic` changing (every count above is a
function of it); `dma_queue.emp` spelling its split other than `blo .split` (the tool fails
loud rather than keeping a stale `0x20000`); `DPLC_Knuckles` gaining a 6-entry frame; or a
sweep window that is not an exact multiple of the boundary, which would turn the invariance
argument in §1 into a windowing artefact.

Re-run with:

```
scripts/provision-aeon-ref.sh /path/to/tree <aeon-rev>
scripts/p6_resweep.py --aeon-tree /path/to/tree --range -65536:65536 --json out.json
```

The raw result for this run — every band of all four subjects, not the summary — is committed
beside this note as **`2026-09-05-p6-resweep-305af222.json`**, so a future comparison has the
band list rather than four aggregate numbers to translate.

---

## 6. Left open, and routed elsewhere

**This parcel moved no aeon bytes and lands nothing in aeon.** The following are reports.

1. **VERDICT C is not swept, and cannot be by this instrument.** The concurrent demand of all
   resident sprite sets is a property of every subject's base *at once*; this sweep moves one
   base at a time. It is 0 of 2 at the current layout. A real re-layout moves several bases
   together and VERDICT C is the bound the 2026-09-03 emulator reading named as the one that
   breaks. **Unpriced.**
2. **The decaying dedup margin (§4) wants a monitor, not a note.** The quantity is
   `packed_data_end`'s distance to the next forbidden band; it is computable by
   `dplc_straddle` today and nothing computes it. Aeon lane.
3. **Knuckles' `2 × 5 = 10` ceiling (§3) is an unstated constraint.** Aeon lane.
4. **P6's row in the inventory should be corrected** on two points: the staleness mechanism
   (bank re-layout → upstream data growth; the banks are *above* `Art_Sonic` and derive from
   it), and item (b), which is now discharged with a null result for the other three subjects.
5. **`Art_Sonic`'s base at `15cb42f7` (`0x72D24`) is derived from the margin identity**, not
   measured from a build at that revision. The two independent predictions in §2 both landing
   is strong, but it is not a listing.
6. Nothing here needs the emulator, so nothing is tagged for foreground runtime follow-up.
