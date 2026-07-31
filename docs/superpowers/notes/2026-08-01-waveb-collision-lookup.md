# Wave-B collision_lookup #1-3 — fused GetType+GetCollision (+ Row80 table adjudication)

**Class PF / effort L.** Census row `2026-07-31-opt-sweep-design.md:116`
(collision_lookup #1-3, "~30%/sensor lever"). Chain len 7 next. First parcel
whose Phase-0 named prerequisite was building a COLLISION-HEAVY drive — the
prior binding drives never exercised the sensor lever.

## Phase 0 — the collision-heavy drive (the named prerequisite)

The OJZ scroll test boots Player_1 in **debug-fly** (yellow square, no
collision); the max-H camera-scroll drive therefore never touches the sensor
lever. Measured BEFORE on the committed `profile_BEFORE_maxH.json`: `Player_Main`
668 cyc/f, `Collision_GetType` **absent from the top 40** — the census's exact
warning.

The lever is unlocked by pressing **B** once (drop debug-fly → physics), letting
Sonic fall onto the OJZ terrain, then holding right. In the 3×3-section test
level Sonic pins against a wall (running left clamps the camera to 0), which is
ideal: it runs the **floor pair AND the wall probe every frame**, deterministic
and stable forever. Committed as `golden/ab/waveb/profile_collision.py` (SKIP_RELOAD,
frame-anchored, prints a grounded/running player-state snapshot to validate the
drive).

BEFORE profile on this drive (s4.debug, 150-frame steady window, 128000 cyc/f budget):

| routine                    | calls/f | cyc/f (incl.) |
|----------------------------|---------|---------------|
| Player_Main                | 1       | 7656          |
| PState_Ground              | 1       | 5551          |
| Player_SensorFloor         | 1       | 2812          |
| **Collision_GetType**      | **6**   | **1956**      |
| Collision_ProbeDown        | 2       | 2312          |
| Collision_ProbeRight       | 1       | 1190          |
| Player_SensorWallDir       | 1       | 1324          |

`Tile_Cache_GetCollision` never appears separately — it was the `jbra` tail-call
target, folded into `Collision_GetType`'s inclusive time. The lookup path is
**1956 cyc/f** (well above the 300-floor stop condition), ~26% of Player_Main.
Per lookup ≈ 326 cyc inclusive (matches a hand cycle-count of the bounds
compares + wrap + ×80 shift-add + fetch).

## Inspection (census re-confirmation — every target re-checked against current source)

- **`Collision_GetType` has ONE caller** (`player_sensors.emp:190`, the `.cell`
  routine). **`Tile_Cache_GetCollision` has NO caller besides** GetType's `jbra`
  tail-call. So the pair is a **private 1:1 chain** — the census's "check for
  other callers of Tile_Cache_GetCollision" resolves to *none*, and the fusion
  is a clean private merge.
- **`Tile_Cache_GetTile` is DEAD** (zero callers; the 2026-07-16 review already
  flagged "Dead export … grepped whole repo"). The census #2/#3 "GetTile ×2
  table" optimizes code that runs zero times per frame — **dropped**.
- **`mul_cache_stride` has 3 sites**: GetTile (dead), GetCollision (the hot
  lookup — folded here), and FillRow `:1440` (cache-fill, already loop-hoisted
  ONCE per call, cold on the collision drive — Sonic pinned, no scroll → no
  fills). The only hot ×80 site is the collision lookup.

## Parcel #1 — the fusion (IMPLEMENTED)

`Collision_GetType` (engine/level/collision_lookup.emp) absorbs the former
`Tile_Cache_GetCollision` body; the latter is deleted from tile_cache.emp. The
merge is **byte-identical in OUTPUT by construction**:

- The `jbra Tile_Cache_GetCollision` tail-call is gone (fall-through).
- `Cache_Left_Col` is read ONCE into `d2` and reused for both the low-bound
  `cmp` and the cache-relative `sub` (no intervening write between the two
  reads). Same for `Cache_Top_Row`.
- Col bounds+convert are interleaved before the row phase; on any air path the
  intermediate `d0`/`d1` are overwritten by `moveq #CTYPE_AIR,d0`, so the
  observable output is identical to the original "check all four, then convert".
- The ×80 shift-add is inlined verbatim (matching `mul_cache_stride`); a
  compile-time `ensure(TILE_CACHE_STRIDE==80)` keeps the stride guard. The two
  `extern()` COLL_* drift guards are NOT duplicated here (tile_cache.emp already
  guards the same invariant) — keeps the port-gate guard count unchanged.

Cross-seam surface change (both port gates updated in lockstep): the fused body
no longer references `Tile_Cache_GetCollision` (pc-rel branch target retired);
it now reads `Cache_Origin_Col`/`Cache_Origin_Row` and loads `Tile_Cache_Collision`
via abs.l `lea`. `collision_lookup_port.rs` + `tranche3_negative_probes.rs`
re-point their synthetic cross-seam sections accordingly; the retired
`Tile_Cache_GetCollision` symbol pin is replaced by the already-pinned
`Tile_Cache_Collision`.

## Parcel #2/#3 — the Row80 build-time table (LOG-AND-SKIP, numbers)

- **#2 (GetTile ×2 table): SKIP outright** — GetTile is dead code.
- **#3 (GetCollision / FillRow ×80 table): LOG-AND-SKIP.** The only hot ×80 site
  is the fused collision lookup (6 calls/f). A 30-entry `coll_row×80` word table
  replaces the shift-add: 40 cyc → ~18 cyc (`add.w d1,d1` + `move.w Tbl(pc,d1.w),d1`),
  saving ~22 cyc/call = **~132 cyc/f**, but GROWS ROM ~60 bytes (the table). ROM
  growth makes it **ineligible for the work-removal clause**, and ~132 cyc/f is
  an order of magnitude below the ~1k bar and below the sub-bar 85% (~850) — it
  fails leg (a) of the three-leg test by a wide margin (the pb#3 precedent:
  sub-bar + no removal = skip). FillRow's `:1440` site is cold on the binding
  drive and already hoisted once/call. **Reopen** only if a drive shows the
  collision lookup at ≫10× the call count (a much larger, slope-dense level).

## Build + A/B results

- **Both shapes build + pack** (B-0 absorbed the region delta): plain
  `crc=5712eb1d len=412329`, debug `crc=7b1f7fd3 len=422170`. **ROM SHRANK 28
  bytes both shapes** (OLD plain 6a69403c/412357, OLD debug 7ea5e77d/422198).
  COLLISION_LOOKUP region 0x30→0x70; TILE_CACHE shrank (GetCollision removed).
- **Profiler A/B (collision drive, s4.debug, OLD 7ea5e77d vs NEW 7b1f7fd3, per-shape lst):**

| routine             | OLD cyc/f | NEW cyc/f | delta | calls O/N |
|---------------------|-----------|-----------|-------|-----------|
| Collision_GetType   | 1956      | 1848      | **-108** | 6/6    |
| Player_Main         | 7656      | 7548      | -108  | 1/1       |
| GameState_..._Update| 27814     | 27707     | -107  | 1/1       |
| VSync_Wait (idle)   | 92715     | 92821     | +106  | 1/1       |
| TouchResponse       | 1188      | 1188      | 0     | 1/1       |

  −108 cyc/f = −18 cyc/call × 6 (the jbra + the two shared cache-word reads).
  Idle VSync absorbs it exactly (+106). Call counts identical → behaviour
  unchanged. `Tile_Cache_GetCollision` gone from both.
- **Non-collision frame class (max-H, debug-fly):** the lever is not called
  there, so that frame class is unchanged (≤ profiler noise) — the "every frame
  class" leg of the work-removal clause.
- **State identity (PS bar) — the wave's cleanest result** (code-point anchor =
  breakpoint at `GameState_OJZScroll_Update` $5E42C — the SAME address both
  builds, the −28 B absorbed by pads below the test code; anchors fc 220/280/340
  on the B-drop collision drive; double-runs per side):
  - **Determinism: OK both sides including the framebuffer hash** — the
    code-point anchor is exact (fc lands on the anchor precisely; the emulator-side
    `state_hash` sees zero run-to-run variance).
  - **VRAM / CRAM / VSRAM / the full VDP register file: byte-identical OLD vs
    NEW at every anchor.** The reg-file hash is CONSTANT across anchors —
    code-point anchoring eliminated the mid-VBlank reg-progress aliasing class
    entirely.
  - **Full 64 KB RAM: exactly ONE differing byte per anchor** ($FFFEFB /
    $FFFEFF — directly below the initial SSP $FFFF00): stale return-address
    low-byte fragments from the previous frame's collision call chain, shifted
    with the code layout (GetType $5E80→$5E40). Zero diffs in engine or game
    RAM — player state, camera, entity arrays all byte-identical.
  - The `fb` hash differs at fc 220/280 (identical at 340): the intra-frame
    beam-phase signature — NEW reaches the anchor ~108 cyc earlier in the frame,
    so the partial-render snapshot differs while every input to rendering
    (VRAM/CRAM/VSRAM/regs) is identical. The classified A3 phase class, not a
    behavior delta.
  - Evidence: `manifest_coll_OLD.json` / `manifest_coll_NEW.json` +
    `coll_{OLD,NEW}_run{1,2}/ram_f{220,280,340}.bin` (captured by
    `ab_collision_state.py`).
- **Protocol find (oracle intel, countersign run):** the code-point runner's
  first live run exposed a wedge — a paused PC sitting ON a breakpoint re-fires
  on `resume` WITHOUT executing, so a "resume until Frame_Counter advances" loop
  spins forever (endless `[sock] resume` in the GUI log, `run_frames` untouched).
  Fix in the runner: single-`step` off the breakpoint PC before re-resuming.
  Also reconfirmed: launching a second GUI while the first is dying wedges the
  new instance's romload at step 6 with `system_running=0` — always
  pkill-and-verify-down BEFORE relaunching.

## Rulings (self-adjudication for the countersign)

1. **Parcel #1 fusion — KEEP under the WORK-REMOVAL CLAUSE.** Byte-identical
   consumer-visible output PROVEN on the oracle (identical call counts + player
   state + settled-frame RAM/VRAM identity); net cost ≤ noise on the
   non-collision frame class; **ROM not grown (shrank 28 B)**; −108 cyc/f on the
   binding collision drive. Exactly the tile_cache #2 precedent (a strict work
   removal keeps on those grounds, magnitude notwithstanding).
2. **Parcel #2/#3 Row80 table — LOG-AND-SKIP** (numbers above): ROM-growing +
   ~132 cyc/f sub-bar + work-removal-ineligible; GetTile dead. pb#3 precedent.

## Premise refinements (honest)

- The census "~30%/sensor lever" is the lookup's SHARE of the sensor cost
  (1956/7548 ≈ 26%), not the optimizable fraction. The **removable** work is ~18
  cyc of ~326 per lookup ≈ **5.5%** — a ~108 cyc/f work-removal, not a ~2k lever.
- `Tile_Cache_GetCollision` had no callers to worry about (private 1:1); the
  census's caller-check resolves to none.

## Step-3 (language/tooling) vs step-5 (engine) findings

**Step-3:**
- The port gates hand-list the synthetic cross-seam sections (the referenced
  RAM/ROM symbols). This fusion changed that set (−Tile_Cache_GetCollision,
  +Cache_Origin_*, +Tile_Cache_Collision) and required parallel edits in two
  test files. An ask: derive the synthetic cross-seam set from the .emp's actual
  link references (the corpus contract closure already knows them) instead of a
  hand-maintained list — it would make a cross-seam-surface change a zero-touch
  test update.
- `mul_cache_stride` is now used only by dead GetTile + FillRow; the collision
  path inlines its own copy (cross-module comptime-fn export isn't wired). A
  shared pub comptime-fn home would remove the inline duplication.

**Step-5:**
- The ROM-neutral alt ×80 form `(row*5)<<4` (32 cyc vs 40) is a byte-identical-
  OUTPUT micro-opt (~48 cyc/f) available at the fused mul site, NOT taken — kept
  parcel #1 a pure structural merge matching the `mul_cache_stride` idiom.
  Available for a future touch (would itself ride the work-removal clause).
- `Tile_Cache_GetTile` is a dead export; removal candidate, but it is the
  `tile_cache` region START anchor (`repin.toml start = Tile_Cache_GetTile`), so
  removing it needs a region-boundary rework. Flagged, not done here.

**Neither-bucket:**
- The Phase-0 finding is the headline: the binding max-H drive is BLIND to the
  sensor lever (debug-fly). Any future collision/sensor parcel must use the
  B-drop physics drive (`profile_collision.py`), now committed.
- The test level is tiny (3×3 sections); Sonic pins against a wall. That gives a
  stable 6 GetType/f but caps the lever's ABSOLUTE magnitude. A larger,
  slope-dense test level would raise calls/f (more two-cell extensions) — it
  would not change the per-call saving, but it would move the Row80 table's
  ~132 cyc/f upward and could reopen #3.
- Oracle instability: the code-point-anchored capture wedged the GUI once
  (ExecuteThread stall after a bus client was killed mid-run); recovered by GUI
  restart + `ab_current.bin` content-swap + SKIP_RELOAD (the tc2/ew1 method,
  flaky loader out of the loop).

## Countersign (overseer, own-run — the porter died on the session limit at the final gate check)

- Fresh builds from the branch reproduce the parcel CRCs exactly: plain
  `5712eb1d/412329`, debug `7b1f7fd3/422170` (chain-6 was 6a69403c/412357 ·
  7ea5e77d/422198). OLD rebuilt from a detached master worktree reproduces the
  chain-6 debug golden byte-exactly (also proving the branch sigil binary is
  compiler-identical to master's — this parcel's sigil changes are harness/
  test-only).
- `refreeze --check`: OK, tip `waveb-collision-lookup`, chain len 7.
- Strict suite own-run: **2861 passed / 0 failed / 4 ignored** (matches baseline).
- Diff review: fusion output-identity re-derived by hand; the retired
  `Tile_Cache_GetCollision` repin symbol consciously re-homed onto the
  already-pinned `Tile_Cache_Collision`; the native.rs measuring-spread widening
  (0x40→0x80, the 0x44 region growth overran the old spread by 4 B in the demo
  layout) is measuring-only under the island/fixpoint rounds — proven by the
  check reproducing all six goldens.
- The committed state-identity evidence is the CODE-POINT capture (the porter's
  earlier frame-quantum capture was superseded and removed; its only finding —
  full-RAM identity mod one dead-stack byte — is subsumed by the stronger run).
- RULINGS CONFIRMED: #1 fusion KEEP under the work-removal clause; #2/#3 Row80
  LOG-AND-SKIP with numbers (reopen on a ≫10× sensor-traffic drive).
