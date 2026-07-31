# Wave-C parallax row-35 — engine-owned per-frame Mode Set 3 re-assert

**Class PF + BA + C3(VDP/timing), effort M.** Census row
`2026-07-31-opt-sweep-design.md:127` + §2.2 "Parallax-hardening (row 35 / t41)".
Chain len 8 next. The standalone hardening vehicle per t41 (STOP-1 survey
`2026-07-29-t41-stop1-row35-survey-and-recon.md`). **OQ-5 RULED: the engine write
includes the DIRECT reg-$0B write (shadow + direct), exactly what the harness
did — behavior-preserving; shadow-only is a ledgered future option, not this
parcel.**

## The parcel

Move the per-frame VDP reg $0B (Mode Set 3) assertion from the OJZ harness into
the parallax engine, then delete the harness force-write.

- **Engine (`engine/level/parallax.emp`):** `Parallax_Update` re-asserts reg $0B
  every frame at `.config_resolved` — the one per-frame site that already holds
  the resolved active config in `a0` (past the null early-out). It derives the
  same mode bits `Parallax_StartTransition.update_mode` computes (per-cell/per-line
  HScroll from the deform tables; per-column V from `pcfg_v_deform_table_bg`),
  writes the shadow + dirty mask, THEN drives reg $0B on the hardware directly with
  a `VDP_CTRL` read to reset the command state machine ahead of the `$8B??`
  register write. `use engine.z80_bus.{stop_z80, start_z80}` added.
- **Harness (`games/sonic4/test/ojz_scroll_test.emp`):** the `:284-328` force-write
  block is deleted (kill row 35); the now-unused `parallax_config` import dropped
  (only `Sec.sec_parallax_config` / `Act.act_parallax_config` fields remain, via
  the Act/Sec imports); the module-header C3 comment updated to record that the
  per-frame mode re-assert is now engine-owned.

### Site choice (justified)

`Parallax_Update` at `.config_resolved` was chosen over a dedicated per-frame proc
because: (1) it REUSES the already-resolved active config — no second selection.
`Parallax_Update`'s Step-1 selection (which decrements `Transition_Frames` and
promotes Target→Current at 0) resolves the IDENTICAL config object that
`Parallax_Active_Config` (and the old harness read of `Transition_Frames` pre-decrement)
selects, for every transition-frame value — proven by case analysis (frames 0 →
Current both; frames 1 → post-decrement promote gives old-Target = new-Current,
harness pre-decrement gives Target = same object; frames >1 → Target both). (2) It
is guaranteed a non-null config there (past `beq .no_config`) and fires for any
band-count (placed before the band-count load, matching the harness's "write for
any non-null config" rule). (3) It runs exactly once per frame and stays after
`Section_UpdateColumns` in the per-frame call order (timing analysis below).

**Adjudicated divergence from the harness (single case):** the harness wrote the
`%10` per-cell default when the active config was NULL (`.mode_default`). The
engine takes the existing `.no_config` early-out and asserts NOTHING on a null
config — parallax is inert, so the HScroll mode is don't-care. Unreachable in
shipped OJZ content (every act section resolves `ParallaxConfig_OJZ_Default`, never
null). The engine's contract "no config → no mode assertion" is cleaner than a
`%10` default that only ever fires when parallax is off.

## Inspection (step-0 re-confirmation against current source)

All §2.2 claims re-verified live before editing:
- `ojz_scroll_test.emp:284+` still carried the per-frame force-write (shadow +
  direct reg $0B with the `VDP_CTRL` command-state reset). Confirmed, deleted.
- `Parallax_StartTransition` (`parallax.emp:186`) remains the sole engine mode
  writer and is edge-triggered (same-config short-circuit at `.recross_current`).
  Left AS-IS — its shadow write is now redundant with the per-frame re-assert
  (step-5 finding), but removing it is out of scope (separate edge write, minimal
  diff). `Parallax_Active_Config` (`:269`) present as described.
- `stop_z80`/`start_z80` (`engine/z80_bus.emp`) clobber only `Z80_BUS_REQUEST` +
  flags — `d0`/`d1`/address regs survive the wrap, so the inline write is safe.

## C3 timing-hazard analysis (where the write lands relative to VBlank + Section_UpdateColumns)

The named risk: the harness wrote during its own update; the engine site may run at
a different point in the frame relative to VBlank and `Section_UpdateColumns`, and a
half-finished VDP address command could be interrupted.

- **Per-frame order is PRESERVED after `Section_UpdateColumns`.** Today the
  harness force-write sits at `GameState_OJZScroll_Update:284`, AFTER
  `Section_UpdateColumns` (`:237`) and `Parallax_CheckBoundary` (`:270`). The
  engine write now lives in `Parallax_Update`, called at `:331` (per frame) — its
  prologue executes AFTER `Section_UpdateColumns` in the SAME frame, ~47 lines
  later than the old harness slot but with nothing new interposed. So the direct
  reg write still lands after any 32-bit address command `Section_UpdateColumns`
  could have left, and the defensive `VDP_CTRL` read still resets that pending-word
  state exactly as before.
- **`Parallax_Update` issues NO VDP commands itself** (it builds RAM buffers —
  `Hscroll_Buffer`, `Vscroll_Factor`, the VSRAM column buffer; the VDP emission is
  the VBlank handler's job). So the direct reg write's in-proc neighbours are RAM
  ops, not VDP commands — the command-state reset guards only against UPSTREAM
  (`Section_UpdateColumns`), which is unchanged.
- **Int-masking posture unchanged.** The write copies the harness's `stop_z80` /
  `VDP_CTRL`-read / `move.w d0,VDP_CTRL` / `start_z80` wrap VERBATIM, with NO added
  `sr` masking. A VBlank landing between the reset-read and the `$8B??` write is
  benign: VInt issues COMPLETE commands (leaving the state machine clean), and
  `$8B??` has bit 15 set → an atomic single-word register write regardless of any
  interleave. Identical to today.
- **The one behavioral ADDITION: two init-time direct writes.** `Parallax_Update`
  is also called during Init (`Parallax_Init`→Update, and the `:182` priming
  Update), so the direct reg $0B write now fires twice during Init — with display
  OFF and `Section_UpdateColumns` (`:159`) already completed (its own `stop_z80`
  wrap closed), so no pending command exists and reg $0B is set to the value it
  would flush to anyway ($02 for OJZ_Default). Provably inert; before the A/B soak
  window; confirmed benign by check (b)'s determinism.

## The two named A/B checks (t41) — BOTH RUN, BOTH PASS

Debug shape throughout (`Debug_Scene_Freeze` is debug-only). OLD = chain-7 debug
`7b1f7fd3/422170`; NEW = this parcel `229446d4/421958`. Code-point anchor =
`GameState_OJZScroll_Update` entry `0x5E42C` — the SAME address both builds (the
region delta is absorbed below the proc entry by B-0). Runner
`golden/ab/wavec/ab_wavec_state.py`, each scene run twice per ROM (determinism);
evidence `manifest_wavec_{OLD,NEW}.json`, pinpoint `{OLD,NEW}.{render340,soak576}.ram.bin`.

### (a) Engine writes the same mode — no render regression on the ojz boot scene — PASS

Scene: boot 60f, hold RIGHT (debug-fly camera scroll → parallax + the mode write
run every frame; `Debug_Scene_Freeze=0`), code-point anchors at fc 220/280/340.

- **VRAM, CRAM, VSRAM, and the full VDP register file (regs) are byte-identical
  OLD vs NEW at all three anchors.** The `regs` hash contains reg $0B itself, so
  OLD==NEW regs IS the "same mode" proof (reg $0B = $02 per-cell H, OJZ_Default,
  both builds). Determinism OK (run1==run2 both sides).
- **Full 64 KB RAM: exactly ONE differing byte, at `0xFFFEFB`** (OLD `5a` / NEW
  `5e`) — a stale return-address low-byte fragment 5 bytes below the initial SSP
  `0xFFFF00`, shifted with the code layout (the call chain addresses moved:
  parallax grew, ojz shrank). Zero diffs in engine/game RAM — camera, player,
  tile cache, parallax state all byte-identical. The classified stack class
  (identical to the collision_lookup #1 countersign's single-byte finding).
- **Framebuffer/screenshot diffs are the mid-VBlank capture-aliasing class, NOT a
  regression.** The `fb` hash and PNGs differ at the anchors; a vblank-scanline
  cmp (`ab_wavec_vshot.py`, `ab_wavec_vcheck.py`) showed the SAME split — at one
  scanline stop (fc256) even VRAM+regs diverged while `Camera_X` was identical,
  because `run_to_scanline 240` catches the VBlank HScroll/tile DMA mid-flight (a
  moving target). At fc195/fc317 the DMA had settled and VRAM+regs matched. This
  is precisely why the campaign anchors on a deterministic CODE POINT, not a
  scanline — the collision countersign's "code-point anchoring eliminated the
  mid-VBlank reg-progress aliasing class entirely." The authoritative evidence is
  the code-point region-hash identity above; the screenshot is a capture-instrument
  artifact of NEW's different per-frame cycle length.

### (b) The extra per-frame write does NOT perturb the Debug_Scene_Freeze cache-fill soak — PASS

Scene: boot 60f, `Debug_Scene_Freeze=1` (`0xFF8A10`) then poke `Camera_X` to a
fixed ascending sequence (0x40/0xC0/0x140/0x1C0/0x240 px), 6f each, driving
`Tile_Cache_Fill` deterministically (frozen camera → `Camera_Update` skipped).

- **VRAM/CRAM/VSRAM byte-identical OLD vs NEW at all five stops.** Determinism OK.
- **Full 64 KB RAM byte-identical at 4 of 5 stops; the fifth (cam_x 0x240) diverges
  by exactly ONE byte at `0xFFFEFB`** (OLD `5e` / NEW `5a`) — the same stack
  return-address fragment, same class as (a). Zero diffs in the tile cache or any
  game RAM. The mode write touches neither tile-cache RAM nor VRAM tiles, so the
  soak's cache-fill is unperturbed. (Both OLD and NEW already ran ONE direct reg
  write per soak frame — the harness did it at `:284`, the engine now at `:331` —
  so this check confirms the RELOCATION, not a net-new per-frame write, leaves the
  soak identity intact.)

## Profiler A/B (the parallax-binding drive: max-H scroll, s4.debug, 120-frame window)

Runner `golden/ab/waveb/profile_drive2.py` (SKIP_RELOAD), OLD vs NEW, per-shape lst.
Both: 16.0 px/frame, `Lag_Frame_Count` delta = **0** in-window (no lag introduced).

| routine                     | OLD cyc/f | NEW cyc/f | delta |
|-----------------------------|-----------|-----------|-------|
| **Parallax_Update** (incl.) | 7246      | 7490      | **+244** |
| GameState_OJZScroll_Update  | 52635     | 52571     | **-64** |
| Section_UpdateColumns       | 6288      | 6288      | 0     |
| VBlank_Handler              | 9614      | 9612      | -2    |
| Tile_Cache_Fill             | 24513     | 24501     | -12   |

- **`Parallax_Update` grows +244 cyc/f** — the relocated mode write (derivation +
  shadow + `stop_z80`/`start_z80` + the direct reg write). This is the honest cost
  of the engine-owned assertion.
- **The whole per-frame update (`GameState_OJZScroll_Update`) NETS -64 cyc/f.** The
  deleted harness block (~308 cyc/f: it redid the config SELECTION + null branch +
  derivation + shadow + direct) cost more than the engine write (+244), because the
  engine reuses the config `Parallax_Update` already resolved. So beyond the
  hardening/scaffolding-removal, the parcel is a small net cycle WIN at the frame
  level.
- **No VBlank-window impact.** The direct reg write is a main-loop `stop_z80`-wrapped
  single-word write, not a VBlank DMA; `VBlank_Handler` is flat (±2, noise). No
  worst-case VBlank wall-time concern (nothing added to the VBlank DMA path).

**Adjudication frame** (per the brief): this is a HARDENING / harness-scaffolding-removal
parcel sanctioned by row 35, NOT a cyc-win parcel — the +244 in `Parallax_Update` is
paid for correctness (the register stays in lockstep with the buffers every frame,
closing the edge-triggered-staleness gap), and it happens to net -64 cyc/f at the
frame level with zero VBlank/lag cost. KEEP.

## Build + gate results

- Both shapes build + pack (B-0 absorbs the region deltas — PARALLAX grows, ojz
  shrinks): plain `7ec2137a/412127`, debug `229446d4/421958` (OLD plain
  `5712eb1d/412329`, OLD debug `7b1f7fd3/422170`). ROM shrank 202/212 bytes.
- Code-point anchor `GameState_OJZScroll_Update` = `0x5E42C` and `Parallax_Update`
  = `0x6A64` are STABLE OLD↔NEW (entry labels unmoved; deltas below them). All RAM
  symbols shape-stable OLD↔NEW (code-only parcel, no RAM-layout change).

## Rulings (self-adjudication for the countersign)

1. **The engine-owned per-frame mode re-assert (shadow + direct reg $0B) — KEEP.**
   Behavior-preserving on the oracle: render inputs (VRAM/CRAM/VSRAM/regs incl. reg
   $0B) byte-identical at 8 code-point captures; full RAM identical mod one moved
   stack return-address byte; the Debug_Scene_Freeze cache-fill soak unperturbed;
   no VBlank/lag impact; net -64 cyc/f. OQ-5 honored (direct reg write included).
2. **Per-frame (not on-active-change) — KEEP the conservative ruled default.** The
   write is unconditional per frame (the ruled default). An on-change gate was NOT
   pursued: it would need a "mode changed" latch and only saves ~244 cyc/f on
   frames where nothing changed — but the shipped scene NEVER changes the mode
   (single config), so an on-change form would degenerate to "write once then never"
   and lose the staleness insurance that is the parcel's entire point. Per-frame is
   both simpler and correct.
3. **Null-config `%10` default — DROPPED** (adjudicated above; unreachable in
   shipped content; cleaner engine contract).

## Step-3 (language/tooling) vs step-5 (engine) findings

**Step-3:**
- The mode-bit derivation now lives in TWO places verbatim — `Parallax_StartTransition.update_mode`
  (shadow only, edge) and `Parallax_Update.config_resolved` (shadow + direct, per
  frame). A shared `comptime fn parallax_mode3_bits(a0) -> ...` (or a small private
  proc) would DRY them; not done here to keep the parcel a minimal, reviewable diff
  and avoid changing StartTransition's write shape. Ask logged.

**Step-5:**
- `Parallax_StartTransition`'s shadow write is now REDUNDANT with the per-frame
  re-assert (Parallax_Update re-derives + writes the same mode the next — and same
  — frame). It could be removed to shrink the edge path, but that is a separate
  behavior-touch on the edge writer; flagged, not done (out of scope, minimal diff).
- The direct reg write reuses the `Parallax_Update` clobber budget (`d0`/`d1`) and
  the already-loaded `a0`; no extra register pressure.

**Neither-bucket headlines:**
- The parcel is a NET frame-level cycle WIN (-64 cyc/f) despite adding a per-frame
  hardware write — because it deletes a MORE expensive harness block that redid the
  config selection. The hardening move paid for itself.
- The screenshot/scanline A/B is the WRONG instrument for this engine (mid-VBlank
  DMA aliasing); the code-point region-hash is the right one. Re-confirmed the
  collision-lookup lesson — future render A/Bs should not chase screenshot cmp at
  scanline/frame boundaries; anchor a deterministic PC and hash VRAM/CRAM/VSRAM/regs.
- t41's "gap is structural-yes / exercised-no" holds: no shipped drive fires a
  mode-changing transition (single config), so the A/B proves the single-config
  regime (all shipped content). The multi-config correctness is by construction —
  the engine derivation is byte-identical to the harness's, which was the reference.

## Countersign (overseer, own-run)

- Fresh builds from the branches reproduce the chain-8 CRCs exactly (plain
  `7ec2137a/412127`, debug `229446d4/421958`); `refreeze --check` OK (tip
  `wave-c-parallax`, chain len 8); strict suite own-run **2861/0/4**.
- Diff review: the engine write is a semantically faithful transplant of the
  harness block (same mode-bit derivation, same shadow+dirty, same
  VDP_CTRL-read command-state reset, same Z80 wrap); the config resolution now
  reuses `.config_resolved`'s already-loaded a0 (the −64 net); order vs
  `Section_UpdateColumns` preserved. The null-config drop is the conservative
  engine contract (no config → no assertion) and unreachable in shipped content.
- Evidence re-adjudicated from the committed files: render check — full VDP
  reg file (incl. reg $0B) byte-identical OLD vs NEW at all 3 code-point
  anchors; RAM identical mod the one moved-return-address stack byte. Soak —
  VRAM/CRAM/VSRAM identical at all 5 stops (the named bar).
- **Classification added for the record:** the soak stops' `regs` hashes differ
  at 3/5 stops OLD vs NEW. This is the established quantum-phase aliasing class,
  NOT a register-value difference: the identical hash values recur ACROSS sides
  at shifted stops (OLD stop-2 == NEW stop-1), i.e. the same finite set of
  mid-VBlank flush snapshots landing at shifted intra-frame phases, exactly the
  A3-classified residue. The authoritative reg-$0B instrument is the code-point
  render check, which is identical. Future soak captures should code-point-anchor
  if reg identity is ever part of their bar.
- RULING CONFIRMED: KEEP — row-35 hardening + harness-scaffolding removal
  (kill row 35 closed), net −64 cyc/f on max-H, zero lag delta, OQ-5 honored.

## Scrolling addendum (overseer, post-merge — closes Volence's live observation)

Volence (watching the emulator): the collision drive's wall-pin means no
scrolling — so it can't exercise parallax deformation. Correct; the render
check's anchors sat on that scene. This addendum re-runs state identity on the
max-H SCROLLING drive (debug-fly + hold right — camera moves every frame,
Section_UpdateColumns + the HScroll/VSRAM build run against a moving world),
comparing the FROZEN chain-7 vs chain-8 goldens directly from git history
(rebuild-from-source of an OLDER chain entry with the NEWER toolchain does NOT
reproduce it — placement state evolves per freeze; the goldens are the
artifacts of record).

**Verdict: VRAM / CRAM / VSRAM / reg-file byte-identical OLD vs NEW at all 3
code-point anchors under active scroll; anchor 280 fully identical INCLUDING
all 64 KB RAM; anchors 220/340 differ by exactly one classified layout-pointer
byte.** The engine-owned per-frame mode write is behavior-identical while the
deform machinery actually runs. Evidence: `ab_wavec_scroll.py`,
`manifest_scroll_{OLD,NEW}.json`, `scroll_*_run*/ram_f*.bin`.

Method note for the record: chain-N goldens are reproducible only by their own
freeze-time toolchain state; for historical A/B sides, extract the golden from
git (`git show <merge>:crates/sigil-harness/golden/…`) instead of rebuilding.
