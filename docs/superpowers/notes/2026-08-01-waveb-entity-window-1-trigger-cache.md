# Wave-B entity_window #1 — the per-section trigger cache

**Class PF / effort M.** Review §3 entity_window High #1; census row
`docs/superpowers/notes/2026-07-31-opt-sweep-design.md:124`. First consumer of B-0b
(RAM packing) — the cache is fresh EntityScanState scratch.

## The problem (measured, BEFORE = max-H drive)

`EntityWindow_Scan`'s section loop calls `ScanRingsRight` AND `ScanObjectsRight` for
every valid window entry (≤4), every frame, regardless of whether anything is entering.
Each call re-loads the ROM ptr + ratchet + origin, advances to the ratchet, reads the
first entry, compares against the right edge, exits. From
`golden/ab/waveb/profile_BEFORE_maxH.json` (128000 cyc/f budget, s4.debug):

| routine                       | calls/f | cyc/f (incl.) |
|-------------------------------|---------|---------------|
| EntityWindow_Scan             | 1       | 5003          |
| EntityWindow_ScanRingsRight   | 4       | 659           |
| EntityWindow_ScanObjectsRight | 4       | 551           |
| EntityWindow_TrySpawnRing     | ~1      | 247           |
| EntityWindow_DespawnRings     | 1       | 2486          |

The two walkers = **1210 cyc/f** across 8 calls; ~247 of that is one real ring spawn
(`TrySpawnRing`), the rest is per-call no-op overhead on quiescent section-lists.

## The mechanism

Add two fresh u16 fields to `EntityScanState` (the review's `ess_*_left_idx` reuse plan
is VOID — those fields were deleted phase2.5 c6):

```
ess_ring_next_x @ $16   // engine-X of the next ring entering from the right ($FFFF = none)
ess_obj_next_x  @ $18   // engine-X of the next object entering from the right ($FFFF = none)
```

`EntityScanState_len` $16 → $1A. `Entity_Scan_State` RAM (`MAX_TRACKED_SECTIONS ×
len`) grows 4×$16 → 4×$1A = **+$10 (16 B)**; downstream engine RAM shifts +$10, even
(B-0b's packing, verified below).

- **Each walker caches, on every exit, the engine-X of the first entry it did NOT
  consume** — the entry at the ratchet (`ess_*_next_x = engineX`), or `$FFFF` at the
  list terminator / null list. The ratchet writes are byte-for-byte the old
  `.update_idx` semantics; only the cache write is new.
- **The section loop gates each call**: `move.w ess_*_next_x(a1),d0 / cmp.w d0,d7 /
  blo .skip`. The walker consumes entries with `engineX ≤ d7` (right load edge); the
  first unconsumed entry has `engineX = cached > d7`. So `d7 < cached` ⇒ calling the
  walker would consume nothing ⇒ **skip is behaviour-identical**. The `$FFFF` sentinel
  makes an exhausted list skip permanently (d7 never reaches `$FFFF`).
- **`InitSection` clears both caches to 0** (alongside the ratchets it already cleared).
  0 < any right edge, so the first post-init/post-slide scan always runs and the walker
  re-establishes the cache. `EntityWindow_Init`'s array clear uses
  `sizeof(EntityScanState)`, so it covers the new fields automatically.

### Provable behaviour identity (the correctness proof for the A/B)

The gate skips a walker call **only** when the walker would spawn nothing. Entity-spawn
TIMING is therefore identical between OLD and NEW: a ring/object at engine-X = X first
loads on the frame where the right edge `d7` reaches X, with or without the gate (the
gate stops skipping exactly when `d7 ≥ X`). **Same entities, same frames, same
buffer contents** — the ROM's runtime behaviour is unchanged, only faster. The A/B can
assert this directly (identical live-entity sets frame-by-frame).

Verifier coverage (review's list): cache written on every walker exit (edge,
terminator, null-list); ratchet semantics unchanged; unsigned `blo`/`bhi` wrap matches
today; `RescanY` reads the ratchet as a bound and touches neither ratchet nor cache, so
the cache stays valid across a vertical re-scan; camera-left motion never retreats the
ratchet, so the right-edge cache stays correct (left entry loads are the slide's job).

### One deliberate deviation from the review (budget-driven, correctness-neutral)

The review suggested `PopulateSectionRings` also seed the cache. It is **not** seeded:
after a slide, `InitSection` leaves the cache at 0, so the first post-slide
`EntityWindow_Scan` runs the ring walker once and re-establishes it from the ratchet.
Slides are ~once per 2048 px, so seeding would buy one skipped walker call per slide —
not worth the ROM bytes (see the placement bound below). Correctness is unaffected: 0
forces the scan, the walker seeds correctly.

## The placement bound that shaped the implementation (honest)

The naïve implementation (separate `.no_list` blocks + a `PopulateSectionRings` seed)
grew the entity_window ROM section by ~0x52 and **tripped B-0's round-0 measuring
spread**: `sections entity_window and children overlap (colliding pins)`. B-0 absorbs a
per-section growth only up to **0x40** (the `+0x40 × rank` cumulative spread;
native.rs `packed_true_bases`). Two correctness-neutral reductions brought it under:
(1) drop the populate seed; (2) fold each walker's `.no_list` into `.at_terminator` by
loading the ratchet before the null-ptr check (writing the unchanged ratchet back is a
no-op). Final growth: entity_window **+0x38 plain / +0x36 debug** (< 0x40, 8-byte
margin). This is the first parcel to approach the B-0 spread bound — a ledger note
(below).

## Build + gate results

- **Both shapes build** (sigil-native): plain `crc=f0d02beb len=412376`, debug
  `crc=29573719 len=422249`. entity_window region len 0x8BA→0x8F2 (plain) / 0xD28→0xD5E
  (debug). RAM +$10, even — **`ram_packing_invariants_{plain,debug}` GREEN** (B-0b's
  guard confirms the RAM stayed even + contiguous under the growth).
- **Owned gates GREEN**: `entity_window_port` (struct-symbol twin updated),
  `repin_pins::pins_rs_is_current` + `generated_pins_match_the_hand_typed_baseline`
  (repin refreshed pins.rs; the one shifted literal, DEBUG_ASSEMBLED_LEN +0x20, updated),
  `ram_packing_invariants`, all non-golden port gates.
- **26 gates RED — all golden-ROM / frozen-table dependent, REFREEZE-PENDING (yours)**.
  The ROM legitimately changed (shared-engine EntityScanState growth shifts sonic4 AND
  the off-canonical demo/config targets). Categories:
  - whole-ROM: `native_full_sonic4_{plain,debug}`
  - golden region byte gates (RAM/ROM immediates shifted): `boot_*`, `parallax_*`,
    `g1_objects_debug`, `p2_{air,ground,spindash}_debug`_region_matches_reference
  - off-canonical anchors/sizes/full: `{config_a,config_b,demo_plain,demo_debug,
    flipped_config_a}_anchor_matches_golden`, `*_full_file`, `*_size_table_rederives_native`,
    `config_b_frozen_placement_exact`, `config_b_doctored_size_table_breaks_the_build`
    (t24 golden), `deform_pointer_equals_placed_label_vma`
  - Spot-checked as clean shifts, not bugs: boot = RAM immediate +0x10 (`b0 74`→`b0 84`);
    native_full = `Section_Init` 0x55CC→0x560C (+0x40 downstream of the grown section).

The refreeze regenerates s4.bin/s4.debug.bin + the six frozen tables + the off-canonical
goldens; pins.rs is already repinned on this branch.

## A/B PLAN (for the overseer's profiler run)

**Primary drive:** the census max-H scroll — `camera_x 2016 → 4416, 16 px/f, 120
frames` (the committed `profile_BEFORE_maxH.json` shape), s4.debug. EntityWindow_Scan
is active there; the churn drive leaves the window inactive (Scan early-outs on
`Entity_Window_Active == 0`), so it is NOT a valid drive for this parcel.

**Anchors (code-point, self + inclusive):** `EntityWindow_Scan`,
`EntityWindow_ScanRingsRight`, `EntityWindow_ScanObjectsRight`,
`EntityWindow_TrySpawnRing`; frame-lag `Lag_Frame_Count`.

**Affected regions to reload/verify:** entity_window ROM region (grew, addresses
shifted); all engine RAM ≥ `Entity_Scan_State` shifted +$10; use the NEW debug ROM
(`aeon/s4.debug.bin`, crc 29573719) built on branch `opt-wave-b2`.

**Expected deltas:**
- `ScanRingsRight` + `ScanObjectsRight` inclusive cyc/f drop from 1210 toward the
  irreducible real-spawn floor (~`TrySpawnRing` 247 + the calls that legitimately run).
  Projected reclaim: **~600-900 cyc/f** of no-op walker overhead.
- `TrySpawnRing`/`TrySpawnObject` unchanged (real spawns still happen, same frames).
- `Lag_Frame_Count` unchanged: `lag_in_window` was already 0 on this drive — the win is
  pure self-time headroom, not lag elimination.
- Behaviour-identity check: the live-entity set (Ring_Buffer + object slots) must be
  frame-for-frame identical OLD vs NEW.

**HONEST bound — this likely lands BELOW the ~1k cyc/f bar (core #1 territory).** The
win ceiling is structural: 4 tracked sections × 2 lists = 8 walker calls/f, each no-op
costing ~100-165c; the gate replaces each with ~20c. So the reclaim is capped at
~8 × ~110 ≈ **~880 cyc/f even when every list is quiescent** — it cannot reach 1k on a
4-section window. On max-H, ~1-2 lists legitimately run each frame, so expect
~600-800 cyc/f.

**Unlike core #1, there is no regression risk:** the gate is a pure skip (~20c) of a
strictly-more-expensive no-op; on fully-active frames it adds only ~8c/gate (≤64c/f,
rare). Worst case ≈ break-even; best case ≈ −880 c/f. So this is a *safe* sub-1k cut,
not a gamble.

**If a ≥1k drive is wanted:** the win maximizes on an active scroll through a SPARSE
entity stretch (long inter-arrival gaps, all 8 lists quiescent) — a max-H run over a
low-density section band. But the per-call no-op cost caps it near ~880 c/f regardless;
no drive on a 4-section window clears 1k from this lever alone. **Recommendation:** if
the measured max-H reclaim is <1k, this is a log-and-skip-per-the-bar OR a
land-anyway-safe-improvement judgment — flagged for your adjudication with the numbers,
exactly as the bar prescribes.

## Ledger candidates / honest bounds

- **B-0 spread bound is now load-bearing.** This parcel is the first to approach the
  0x40/section round-0 measuring spread; it fit only after two size reductions. A future
  PF parcel that must grow a single ROM section by >0x40 needs a hand ruling or a
  spread-mechanism widening (sigil-side). Logged for the census.
- **The hand-typed `generated_pins_match_the_hand_typed_baseline` literals are pin-tax**
  the B-0 ruling's own logic (it retired the sibling `secondary_pin_classes_*` for
  exactly this) would retire — DEBUG_ASSEMBLED_LEN moved and had to be hand-edited. A
  step-3 ask: fold its surviving literal-value asserts into the generator-vs-file check,
  or retire it like its sibling. Deferred to you (relitigating a ruling).
- The 4-section window (`MAX_TRACKED_SECTIONS`) is the win ceiling. A wider window would
  raise both the cost AND the reclaim, but that is a separate design axis.

## Overseer A/B results + adjudication (countersign)

**Profiler (max-H drive, s4.debug, OLD golden e370f73c vs NEW 29573719, per-shape
listings):** whole-loop `GameState_OJZScroll_Update` **−896 cyc/f** (idle +899);
`EntityWindow_Scan` self 5003→4089; the right-walkers' separately-attributed time
went to zero (the gate skips the no-op calls; `TrySpawnRing`/`DespawnRings` cycles
identical — spawn activity unchanged). Near the porter's structural ceiling because
the window slides only once per 16 frames even at max scroll — the skip pays on the
15 non-sliding frames.

**State identity (plain shape, quantum anchors fc 234/414/694, runs duplicated,
determinism OK):** VRAM/CRAM/VSRAM identical at every anchor; VDP regs identical at
2/3 (the 414 delta = the established mid-VBlank register-sequence capture class).
RAM compared zone-aware (the scan-state is a 4-element array, each +4; vars after it
shift +0x10; the stack page is fixed): below-array IDENTICAL · every element's OLD
fields IDENTICAL · ALL shifted variables IDENTICAL · stack = the established
dead-residue class + one interrupted-PC byte (`22D0`/`22D4`, the A3 signature).
The cleanest state result of the wave.

**The countersign's own catch (fixed before the freeze):** the first freeze attempt
moved the DEBUG sound bank +0x20 — the mt gate caught a B-0 edge case where the
LABELED bank head lost its hard-org anchor once the pre-bank blob's untrimmed
align-pad image crept past `$58000` (this parcel's +0x40 debug growth crossed the
threshold; plain's +0x38 did not). Fixed in `packed_true_bases`: the phase-bank test
now precedes the labeled test (bank content NEVER packs — the Z80 side holds pointers
into it), and the fixpoint re-measure scratches the banks like `phase_region_mask`
does (the position-stale align pad otherwise collides with the hard org at shifted
bases). Both banks byte-identical to the master goldens after the fix; the re-run
profiler on the fixed debug ROM: `EntityWindow_Scan` −916, whole-loop −917 (idle
+920) — the plain-shape state A/B transfers byte-exactly (plain never shifted).

**Ruling: KEEP at −916 (sub-bar).** The ~1k bar is explicitly approximate; this
parcel keeps because it passes ALL THREE of: (a) measured ≥ ~85% of the bar on the
binding drive; (b) behavior-identical by construction with a BOUNDED adverse case
(≤64 cyc/f fully-active); (c) the win measured near ceiling on the binding drive —
no unmeasured-drive dependence. The test is the precedent: core #1 fails (b)/(c)
(measured regression), pb#3 fails (a) by an order of magnitude. A sub-bar parcel
failing any leg skips.
