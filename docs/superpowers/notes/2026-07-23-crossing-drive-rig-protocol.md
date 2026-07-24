# Crossing-drive rig — protocol + standing capability (parallax transition parcel, step 0)

The deterministic oracle choreography that drives the parallax transition state
machine (`Parallax_StartTransition` → `Parallax_Update` promote/lerp) so the
boundary-crossing bugs (B6/B3/B1) can be reproduced and A/B-verified on the
canonical ROM. Built once; reused for every transition fix. **Poke-driven,
ROM-preserving, wedge-avoiding** (per the row-1408 binding technique: drive
state via memory pokes, NEVER held-input-through-a-breakpoint).

## Boot (deterministic, no Game_Entry flip needed)

The OJZ horizontal-scroll test scene is the **default boot scene**. After reset
+ a few seconds of run, `Game_State (0xFF8004) == GameState_OJZScroll_Update
(0x5E42C)` and parallax is live with `Parallax_Current_Config == OJZ_Default`,
`Target == 0`, `Transition_Frames == 0` — a clean steady state.

Use the **debug** build (`s4.debug.bin` / `s4.debug.lst`): it carries the
symbol table + `Debug_Scene_Freeze`. Always hash/size-verify the loaded ROM
against the freshly-built file before trusting any address (genesis-dev
"verify the artifact"). Canonical debug = crc32 `80d14183`, 429134 bytes.

## The freeze

`Debug_Scene_Freeze (0xFF8A10) = 1` makes `GameState_OJZScroll_Update` skip
`Camera_Update` **and** `EntityWindow_Scan`. It does **not** gate
`Parallax_CheckBoundary` or `Parallax_Update` — those run every frame. So with
the camera frozen the transition state machine still advances each frame, while
`CheckBoundary` sees no crossing (camera stationary) and does not disturb a
staged transition.

## Address table (debug build)

| symbol | addr | notes |
|---|---|---|
| `Parallax_Current_Scroll_A` | `0xFF8896` | ds.w 8 (bands), Plane A (hard-locked = −camX) |
| `Parallax_Current_Scroll_B` | `0xFF88A6` | ds.w 8, Plane B (lerps during transition) |
| `Parallax_Current_Vscroll_BG` | `0xFF88B6` | |
| `Parallax_Current_Config` | `0xFF88B8` | ptr (4) |
| `Parallax_Target_Config` | `0xFF88BC` | ptr (4); nonzero = smooth transition staged |
| `Parallax_Transition_Frames` | `0xFF88C0` | u8; counts down, 0 = stable |
| `Parallax_Snap_Pending` | `0xFF88C1` | u8; 1 = next Update snaps current←target |
| `Hscroll_Buffer` | `0xFF850A` | 896 B (224 lines × 4); rewritten every frame in per-line mode |
| `Camera_X` | `0xFFA140` | 16.16; high word = world px |
| `Debug_Scene_Freeze` | `0xFF8A10` | u8 |
| `Parallax_Update` | `0x690E` | proc entry (breakpoint target) |
| `Parallax_StartTransition` | `0x6834` | proc entry |
| `ParallaxConfig_OJZ_Default` | `0x11428` | band_count 4, BG FACTOR_1_2, per-line, smooth |
| `ParallaxConfig_OJZ_Windy` | `0x1156C` | band_count 1, per-line, smooth |

(Re-derive from the fresh `.lst` after any rebuild — symbol addresses shift with
byte-changing edits.)

## Frame-step method — and the sit-on-breakpoint GOTCHA (new reusable caution)

Breakpoint at `Parallax_Update` (`0x690E`). To advance **exactly one frame**:

1. `step 1` — move PC OFF the breakpoint address (into the proc body).
2. `resume`
3. `wait_for_break` — lands at the next frame's `0x690E`.

**GOTCHA (banked like the §D wedges):** calling `resume` while the CPU is sitting
*exactly on* the breakpoint PC re-hits the breakpoint **without executing the
proc body** — the frame does not advance and no state changes, but it *looks*
like it broke normally. Symptom that caught it here: staged pokes (camX, Snap)
appeared un-consumed after a "frame." Always `step 1` off the breakpoint before
`resume`. (Also: the FIRST `wait_for_break` after adding a breakpoint can report
a lagged PC; the next resume/wait pair lands cleanly.)

## Staging a faithful smooth transition

Two equivalent triggers, both replicating `StartTransition`'s smooth branch:
- **Execute** `Parallax_StartTransition` with `a0 = configB` (sets Target,
  Frames=`PARALLAX_TRANS_DEFAULT`=16, and the VDP mode-3 shadow), or
- **Poke** the smooth-branch state directly: `Target_Config = configB_addr`,
  `Transition_Frames = 16`. (Omits only the mode-shadow write — irrelevant to
  B6/B3; relevant to B2, where the execute form is preferred.)

`Update` then decrements Frames each frame, lerping Plane B from Current toward
Target's band factors; when Frames hits 0 it runs the promote path.

### Why a config CHANGE must be staged (not camera-driven)

Every OJZ act1 section has `sec_parallax_config = 0` (all inherit
`ParallaxConfig_OJZ_Default` — the old per-section fixtures were superseded by
the Deep Forest BG). So a real camera crossing fires a boundary but **no config
change** → `StartTransition` no-ops → the bugs never manifest from camera-driving
alone. The rig stages the config change directly (the poke IS the config change),
which is the faithful trigger given the shipped data.

### Engineering a B3 lerp gap

The production configs available did not yield a clean band-factor gap through the
built band-entry decode (OJZ_Windy's band-0 BG target read equal to Default's at
the test camX — a data property, not chased). Since **B3 is a property of the
lerp/snap MATH (config-independent)**, the gap is engineered by offsetting
`Parallax_Current_Scroll_B[band0]` before the transition — faithfully representing
a band caught mid-lerp. The lerp/promote CODE exercised is the production path.
(B6 needs no gap at all — see below.)

To settle a legible baseline: poke `Camera_X` to a mid-section value (e.g. 1024px
= write `0x04000000`), set `Snap_Pending = 1`, advance one frame → Scroll settles
to the config's steady state at that camX (FACTOR_1 → −1024, FACTOR_1_2 → −512).
Keep `camX + 160 < 1888` so `CheckBoundary` stays in section 0 and does not fire.

## Observables

### B6 — promote-frame rebuild skip (sentinel-overwrite technique)

The definitive, config-agnostic proof:
1. Settle a clean no-transition baseline.
2. **Control:** sentinel-fill `Hscroll_Buffer` ends (e.g. 16 B of `AA` at
   `0xFF850A` and at `0xFF887A`); advance one *normal* frame → sentinel is
   OVERWRITTEN with real HScroll data (`FC00 FE00…`). Proves the fill runs and
   the sentinel detects it.
3. **Test:** stage `Target = OJZ_Default, Frames = 1`; re-sentinel; advance one
   frame (the promote frame, Frames 1→0) → sentinel **SURVIVES** ⇒ the whole
   Step5+Step4+fill rebuild was skipped. Confirm `Current_Config` promoted,
   `Target = 0`, `Frames = 0` (promote logic ran, but `beq .no_config` was taken
   from the `move.l #0,Target` Z-clobber).

**RESULT (canonical debug ROM, this session): B6 CONFIRMED.** Control overwritten;
test sentinel survived at both buffer ends; Current promoted / Target cleared /
Frames 0.

### B3 — geometric lerp residual + promote-frame snap

Watch `Parallax_Current_Scroll_B[band0]` across the 16-frame window. Verified this
session: one lerp step moves current by `(target − current) >> 4` (e.g. −768 →
−752 at gap 256). The residual after the window (~(15/16)^16 ≈ 0.356 of the gap)
snaps in one frame — and B6 delays that snap by one frame (the promote frame skips
the band loop entirely, so the snap lands the frame after). B6 must be fixed first.

## Rig state after a run

Leaves the scene frozen (`Debug_Scene_Freeze = 1`), a breakpoint at
`Parallax_Update`, and possibly a staged/mid-transition state + poked camX. To
return to live play: clear the breakpoint, `Debug_Scene_Freeze = 0`, and reset,
or reload the ROM. For a fresh A/B run, reset + re-derive addresses from the
current build's `.lst`.
