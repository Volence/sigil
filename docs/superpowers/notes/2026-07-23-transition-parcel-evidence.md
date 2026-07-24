# Boundary-crossing transition parcel — rig A/B evidence log

Per-fix before-repro + after-clean, driven by the crossing-drive rig
(notes/2026-07-23-crossing-drive-rig-protocol.md). Canonical debug ROM used for
live proof; both shapes rebuilt fresh per fix.

Baseline canonical (pre-parcel): plain `00f609a5`/421089 · debug `80d14183`/429134.

---

## B6 — promote-frame CC-clobber (rebuild skip) — CLOSED

**Bug:** `Parallax_Update` promote path ended with `move.l #0,Target` (Z=1 from
its immediate source), so `.config_resolved: beq .no_config` was taken on every
smooth-transition promote frame → entire Step5+Step4+fill rebuild skipped →
Hscroll/Vscroll keep the previous frame's contents = one-frame parallax freeze.

**Fix (both twins, length-neutral reorder):** move the `move.l d0,Current` to be
the LAST of the three promote writes, so `.config_resolved` reads Z from d0 — the
same "active config in d0, Z reflecting it" invariant that the `use_target` /
`use_current` paths already satisfy. `parallax.emp` :366-373, `parallax.asm`
:229-236.

**Rig A/B (Hscroll_Buffer sentinel-overwrite, config-agnostic):**
Setup: OJZ scene, `Debug_Scene_Freeze=1`, camX poked 1024, baseline settled
(Scroll_B −512), stage `Target=OJZ_Default, Frames=1`, sentinel `Hscroll_Buffer`
ends (`AA` ×16 at `0xFF850A` and `0xFF887A`), drive the single promote frame.

| | promote-frame Hscroll_Buffer (both ends) | Current_Config | Target | Frames |
|---|---|---|---|---|
| control (normal frame) | overwritten `FC00FE00…` | — | — | — |
| **before-repro** (canonical `80d14183`) | **`AA…AA` survived** (rebuild SKIPPED) | promoted `0x11428` | `0` | `0` |
| **after-clean** (fixed `7460a0c2`) | **`FC00FE00…`** (rebuild RUNS) | promoted `0x11428` | `0` | `0` |

Promotion completes correctly in both (Current←Target, Target cleared); the fix
only restores the rebuild on that frame.

**Scope class:** byte-CHANGING, **length-NEUTRAL** (pure reorder — same three
opcodes). Both shapes keep size + `EndOfRom` (`0x5DB60`): plain 421089, debug
429134. So NO region-slide ripple (controllers..sound_api bases, engine.inc
orgs, repin all unchanged). New canonical (fresh dual builds): plain
**`bb5ddc5a`**/421089 · debug **`7460a0c2`**/429134.

**Gate:** full paired strict **2488/0** (SIGIL_STRICT_GATE, AEON_DIR=branch tree).

---

## Window-slide mask-migration observation — CLOSED (observed; value-audit deferred)

The carried Phase-2.5 rider: observe one real `EntityWindow_Slide` +
`Entity_Loaded_Masks` migration live. Driven per the row-1408 binding technique
(scroll-target, not held-input, not freeze — `Debug_Scene_Freeze` skips
`EntityWindow_Scan`).

**Drive:** OJZ scene UNFROZEN; poked the scroll target (`Player_1` x_pos
`0xFF8A14` forward) so `Camera_Update` chases the camera across the sec-0→1 X
boundary; breakpoint on `EntityWindow_Slide` (`0x4824`). Note: the 2×2 window
(`MAX_TRACKED_SECTIONS=4`, Active `0x0F`) on the 3-wide grid does NOT slide in X
until the camera CENTER reaches sec 2 — a crossing into sec 1 alone keeps the
2×2 corner at 0. Pushed the camera center toward sec 2 to trigger it.

**Observed (real slide fired):**
- Anchor **(0,0) → (1,0)** — single-axis (X moved, Y unchanged). The DEBUG
  single-axis-invariant `assert.w` (BuildEntries→MigrateMasks) did **NOT** fire
  → invariant holds live.
- Snapshot (`Entity_Mask_Scratch`): old entry ids = sections `{00,01,03,04}`
  (old 2×2), old ring masks `{7F,01,3F,01}`.
- New window (anchor (1,0)) entries → sections `{01,02,04,05}` (read from
  `Entity_Scan_State` ess_section_id, stride 0x16, +0x12); new ring masks
  `{3F, 01, F00F, 00}`. Active preserved `0x0F`. Migration ran; block
  reorganized coherently (4 slots, valid mask preserved).

**Value-audit deferred (NOT a finding).** The camera was driven as a
discontinuous teleport (poked Camera_X + Player across a ~3800px jump), so
PopulateSectionRings/despawn ran on artificial intermediate frames — the
per-section loaded-bit VALUES reflect the poked motion, not natural play (e.g.
new sec1 = 0x3F is plausibly sec1's own rings loading as the camera entered it;
small ring counts alias by coincidence with the evicted sec3's 0x3F). Cleanly
judging migration value-correctness needs a natural 16px/frame continuous scroll
(~120 frames) — a deeper entity-window audit, out of this parallax parcel's
scope. The subsystem's own guard (the single-axis assert) passed. **Rider closed
with a positive live observation; value-audit left as an entity-window
follow-up if ever demanded.**

## B2 — mode-contract (active-config coherence) — CLOSED

Gate ruling (2026-07-23): **Option B** (single `Parallax_Active_Config` accessor =
Target while Transition_Frames>0; route consumers #4/#5 through it) + **sub-decision
(i)** (fix only #4/#5; ledger the engine-owned mode-register write + kill-list the
harness force-write) + **rig-only fixture blessed** (3 constraints). Design note:
notes/2026-07-23-b2-mode-contract-design.md.

**Fix (both twins + buffers.asm):** new `Parallax_Active_Config` proc
(parallax.emp/.asm :~274) returns d0=active config, Z reflecting it. Routed
`Vscroll_Write` (parallax.emp :304, `bsr.s`) and the HScroll DMA-length select
(`buffers.asm` :168, `jsr` — no `.emp` twin) through it. The band builder, fill
format, and mode-set-3 register already commit to Target@frame-0; this aligns the
two stragglers.

**Scope class:** byte-CHANGING + length-changing (+0x10 parallax: accessor +0x12,
Vscroll routing −0x2). Ripple (5-site doctrine): `repin` → pins.rs (PARALLAX len
+0x10, SOUND_API base +0x10, 3 SOUND_* pins); `engine.inc` 2 resume orgs (parallax
+ sound_api, +0x10 both shapes, HAND); `repin_pins.rs` SOUND_API-base baseline
(HAND, delta-chain entry); `mixed_dac_rom.rs` UNCHANGED (no sound-content ref);
`repin.toml` UNCHANGED (no region added). EndOfRom `0x5DB60` unchanged (absorbs in
padding). New canonical (fresh dual builds): plain **`c74eb070`**/421133 · debug
**`8ecbf24e`**/429176.

**Rig A/B (constraint c) — the rig-only fixture.** Fixture = a copy of the real,
macro-built `ParallaxConfig_OJZ_Default` (band_count 4) written to scratch RAM
(`Entity_Mask_Scratch` 0xFFAD20, safe with the camera frozen) with `deform_table_bg`
nulled → a well-formed **per-cell** config. Poked-pointer only: **zero fixture bytes
in ANY ROM** (constraint b — RAM-resident, `Target_Config` poked to it; constraint a
— derived from a real-struct config by a documented field flip, not raw bytes).
Staged Current=OJZ_Default (per-line) → Target=fixture (per-cell), Frames=16.

HScroll DMA-length observable (breakpoints on the two enqueue paths, read in the
window at Frames=15):

| ROM | buffers HScroll path during window | vs the per-cell builder |
|---|---|---|
| **before** (B6 `7460a0c2`, reads Current) | **LINE 0x21FC → Static_Hscroll_Line 896 B** | MISMATCH = the ≤16-frame tear |
| **after** (B2 `8ecbf24e`, reads active=Target) | **CELL 0x2218 → Static_Hscroll_Cell 112 B** | MATCH = coherent |

Both at Frames=15 (in the window), Target=0xFFAD20 (fixture), Current=0x11428
(OJZ_Default) — verified each side.

VSRAM-stride observable (consumer #5): `Parallax_Active_Config` is a PURE function
of global state (Transition_Frames + Target/Current), independent of caller, so the
value it returns for buffers (proven = Target in-window) is the same value
Vscroll_Write reads in the same frame. The before/after listing diff confirms
Vscroll_Write's source flips `move.l (Parallax_Current_Config).w,d0` (before, :68A2)
→ the `Parallax_Active_Config` call (after) — the identical one-line routing. So
Vscroll_Write's whole-plane/per-column VSRAM decision follows Target during a
transition (coherent), vs Current before. The H-path is the representative live A/B;
the V-path shares the proven pure accessor + the same routing diff.

**Shipped-config invariance (gate-required):** for shipped (mode-equal) config pairs
Active and Current select the SAME mode, so `Parallax_Active_Config` returns a config
whose mode bits equal Current's — buffers/Vscroll make the identical decision they
made pre-B2. Shipped rendering is provably unchanged. (Doubly so: shipped play fires
NO transition — all OJZ sections share config 0 — so Transition_Frames is always 0
and the accessor returns Current verbatim.)

**Gate:** full paired strict **2488/0** (SIGIL_STRICT_GATE, AEON_DIR=branch tree),
failures-first, ripple resolved.

## B3 — frames-remaining ramp — (pending; inside B2's state machine)

## B1 — re-cross cancel branch — (pending; inside B2's state machine)
