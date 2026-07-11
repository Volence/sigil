# `table` adoption survey — the full ported corpus (2026-07-11)

Requested by Volence ("what would our new table construct be good for, from
the .emp files we ported"). Scope: all 24 ported `.emp` files. This
supersedes-and-confirms the step-6 sweep verdicts recorded at ddb481e with
a full-corpus look (the sweep assessed the four data files; this walked
everything).

## Headline

**The ported corpus is CLEAN — zero retrofit candidates remain.**
sfx_bank was the one true fit and it's already converted (197 → 98 lines,
−50%; adding an SFX = one row). Everything else is either not data, data
that a DIFFERENT construct already covers better, or a shape `table`
deliberately doesn't do yet. That's the healthy outcome: it means the
construct inventory (offsets / dispatch / constructors / table) is
partitioning the data-shape space instead of overlapping it.

## Per-file verdicts

**Retrofitted (1):**
- `sfx_bank.emp` — `table SfxTable` (sparse `{id: cell}`, 9 rows over the
  $33..$B9 id range, 126 holes auto-filled). The acceptance target; done.

**Covered by a better construct (3) — do NOT convert:**
- `sonic_anims.emp` (11 animation scripts) + `particle_anims.emp` (1) —
  `offsets` is the exemplar tool for animation bytecode (self-relative
  word table + variable-length bodies). `table` would add nothing.
- `act_descriptor.emp` — 9 dense `Sec` records built by the validating
  `ojz_sec()` constructor. The constructor VALIDATES; `table` doesn't.
  Converting would lose checking to save nothing.

**Table-gap (2) — shapes table can't express yet, both ledgered:**
- `mt_bank.emp` — DEBUG-conditional members + a parallel patch table with
  a non-1:1 cell (DrumTest → MovingTrucks_Patches). Needs
  dense-conditional-multi-cell + parallel-side-table modes. Demand: 1
  file, 1-3 rows — stays a ledger row until a classic-Sonic zone/level
  table makes it worth building.
- `rings.emp`'s Ring_Buffer — a RUNTIME structure (RAM, spawn-time
  mutation), not a comptime collection. Out of `table`'s domain by
  definition, listed here so nobody re-asks.

**No data at all (18):** the engine code files (core, collision, animate,
dplc, aabb, collision_lookup, sound_api, sst, types, constants,
controllers, game_loop, hblank, vdp_init, math¹, test_particle,
test_solid, and dac_samples²).
¹ math.emp's Sine_Table is a dense binary lookup blob (`sine.bin`), one
value per angle — an include, not a keyed collection.
² dac_samples' descriptor table lives .asm/Z80-side (cross-seam), like
the sfx win-tab.

## Where table's REAL future demand is (not in this corpus)

The construct was born general on purpose; the demand queue is in
UNPORTED material:
- **PLC lists + the six back-patch macros** (count-header record lists —
  named in the D2.36 design as the record-list mode's target).
- **Level data tables** as the per-act split lands (art/layout/collision
  pointer sets — id-keyed, sparse over level slots).
- The S3K frequency roadmap's collection shapes (see
  [[emp-data-table-dsl-candidates]] memory / the D2.36 spec §1 demand
  set).

So the practical guidance for future ports (this is the step-4 "adopt"
checklist line): reach for `table` when you see a sparse id→cell bank, a
count-header record list, or a sentinel list; reach for `offsets` for
ordinal jump/bytecode tables; keep validating constructors for dense
fixed-shape records; and if you hit conditional members or parallel
tables, that's the mt_bank gap — ledger the demand, don't force it.
