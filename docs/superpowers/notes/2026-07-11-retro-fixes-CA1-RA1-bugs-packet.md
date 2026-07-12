# Retro-fix batch — C-A1, R-A1, comment/doc, F3, Bug 1 (camera), Bug 2 (push)

Date 2026-07-11. One batch of queued retro findings + two gameplay bugs.
Full workspace strict **2142/0**, clippy clean, both aeon shapes build clean,
`repin --check` = no drift. Branch NOT merged (Volence's gate).

## Byte-neutral (no re-pin) — tasks 1 & 2

- **C-B1** core.emp InitObjectRAM free-stack comment: was backwards. Truth:
  push is LOW→HIGH (slot 2 first … slot 41 last); LIFO pop → highest slot
  (41) allocated first, pool fills downward. Comment fixed both .emp + twin.
- **C-B3** core.emp AllocEffect: added the *why* for skipping slot_tag —
  slot_tag is the entity-window quadrant index, read ONLY by the Y-despawn
  band; effects are never entity-window-managed, so it's never read.
- **F1** types.emp `Radius` doc: said "half-extent", verified FALSE against
  `aabb_axis_test` (sums both FULL dims, compares vs TWICE the centre
  distance). Doc corrected to "full dimension". **NAME MISMATCH flagged to
  Volence** ("Radius" reads as half-extent) — NOT renamed unilaterally.
- **F2** sound_api.emp Sound_PlaySFX header: split ENFORCED (d1/a0, the
  `preserves()` movem contract) from INCIDENTAL (a1/d2-d7/SR — current body
  just never writes them, not a guarantee). Mirrored to twin.
- **A-B2** animate flip-mask: named `$F9 = ~(RF_XFLIP|RF_YFLIP)` /
  `$06 = RF_XFLIP|RF_YFLIP` (status→render_flags facing propagation, bits
  aligned). Mirrored to twin.
- **F3 (recheck)** — BUILT `z80_bank`/`z80_window` comptime-fn templates in
  sound_api.emp for the `(addr & $7F8000)>>15` / `(addr & $7FFF)|$8000`
  Z80-window address derivation (3 call sites in Sound_PlayMusic). Byte-neutral
  (sound_api_port both shapes green). Twin keeps raw instrs (like
  stop_z80/start_z80). Controllers P1/P2 latch: **LOOKED AT, DECLINED** — the
  6-instr latch block (2×) would need the template to reference the proc-local
  `.read_pad`, which hygienic template labels can't resolve (the known
  cross-fragment label-scope gap that blocked emit_piece_loop). File is clear
  at 66 lines; win marginal, mechanism hazardous. Ledgered.

## Byte-changing (re-pin) — task 3

- **C-A1** core: `bne.w RunObjects_Frozen` → bare `bne`; two
  `ifdebug bsr.w Debug_AssertObjLoop` → `jbsr`. Outlawed byte-lock comments
  DELETED. **FINDINGS:**
  - The deleted comment was not just stylistically outlawed — it was
    FACTUALLY WRONG. It claimed the bsr sites would "relax to .s"; in fact
    Debug_AssertObjLoop sits *near* the loops (disp $48/$0A) so jbsr DOES pick
    .s (they shrink −2 each, DEBUG only) — the comment's premise ("stays
    explicit") was the opposite of reality.
  - The `bne` is SHAPE-DEPENDENT: plain disp $7C fits .s (−2), but the DEBUG
    shape's two bsr sites push RunObjects_Frozen out to disp $1A6 → forces .w.
    Sigil's bare `bne` width-selects per shape correctly. The AS twin can't use
    a bare branch (bare branches PHASE-ERROR against the tight adjacent
    `bsr.s .run_always` — corrupts its disp to an illegal 0), so the twin uses
    an explicit `ifdef __DEBUG__ / bne.w / else / bne.s` pair. Net: core LEN
    −2 plain / −4 debug.
- **R-A1** rings DrawRings cull: `addi #16` → `addi #8` at both cull sites
  (SIZE-NEUTRAL — immediate only). The ring sprite is CENTRE-anchored
  (top-left = centre−8), so the correct bias is +8 (not +16); the compares
  stay `336`/`240` (screen + full sprite width 16). Old +16 over-shifted and
  culled rings ~8px early on the right/bottom. Math derived + confirmed by the
  byte gate. Comments rewritten. Base slid with core; LEN unchanged.
- Re-pin: `repin` moved 17 pins (core −2/−4; object regions slid; sound_api
  +4/+2 because camera Bug-1 grew ~+6 and links after the object regions).
  Pasted 7 resume orgs into engine.inc. Updated hand-typed baselines in
  repin_pins.rs + the game_loop disp bytes in mixed_dac_rom.rs.

## Gameplay bugs (aeon .asm, not in the port corpus) — tasks 4 & 5

- **Bug 1 — camera jump-fall lock** (camera.asm GAME_CAMERA_JUMP_LOCK): gated
  the lock on y-velocity so it holds only while RISING/HANGING (the stated
  intent) and releases once y_vel > 0 (falling). **LIVE-VERIFIED (oracle,
  deterministic):** same setup (JUMP state, player below focal, dist 80 —
  inside the lock zone), only y_vel sign differs → FALLING: camera_Y 0→32
  (follows down); RISING: camera_Y 0→0 (holds). Pre-fix would lock in both
  (the jitter). CAM_SCREEN_HALF_H bail-out kept as failsafe.
- **Bug 2 — push-anim flicker** (player_ground.asm:667 `bmi.s`→`ble.s`): flush
  contact (dist==0) now counts as still-pushing (keeps ST_PUSHING) instead of
  falling through to .clear_push. The dist==0 path through .wall_hit is inert
  (asl of 0 backs out 0 vel, gsp killed = stops the subpixel creep, facing
  logic runs). Verified ST_PUSHING is set ONLY at player_ground.asm:713 (the
  terrain probe) — no separate TestSolid push-bit path, so the single fix
  covers all cases. Builds clean; **live push-flicker repro DEFERRED** — the
  object-test harness (debug-fly default over bottomless pits) can't cleanly
  stage a grounded wall-push; recommend confirming in a normal playable level.

## Follow-on (2026-07-12): screen-dim constants + S3K ring art port

- **SCREEN_WIDTH/HEIGHT in the ring cull** (byte-neutral) — Volence asked why
  `224`/`320` were magic. They already existed (`constants.asm`/`constants.emp`
  `SCREEN_WIDTH=320`/`SCREEN_HEIGHT=224`, drift-guarded, used by entity_window);
  the ring cull just wasn't using them. Rewrote the cull as
  `addi #RING_WIDTH/2` (centre→edge bias) + `cmpi #SCREEN_WIDTH+RING_WIDTH` /
  `#SCREEN_HEIGHT+RING_HEIGHT` — same immediates, byte-neutral (rings_port green),
  both sides. (Also noted raw `#224` in parallax.asm:639 / section.asm:479 —
  un-ported files, flagged not swept.)

- **S3K ring art port — DONE (pragmatic 4-frame swap, Volence-approved).**
  The aeon ring was a 1-tile placeholder; now real S3K art with a working spin.
  - Extracted skdisasm Ring.bin (Nemesis, 14 tiles via clownnemesis-tool),
    composed 4 square 2×2 frames (16 tiles) in VDP column-major order:
    F0 full / F1 narrower / F2 thin-edge (S3K's 1×2, centred into 2×2 with a
    +4px shift) / F3 = F1 H-flipped (baked). Palette remap S3K line-0
    {1,5,6,F}→sonic.bin line-0 {F outline, E gold, 6 white glint}. Script +
    `games/sonic4/test/ring_art.bin` (512 B) committed.
  - VRAM: ring region $3E8 expanded 4→16 tiles ($3E8-$3F7, fits Zone Pool A);
    `VRAM_TEST_MARKER` relocated +4→+16 ($3F8). `object_test_state.asm` ring
    art = BINCLUDE (TestArt now 24 tiles). `VRAM_RING_PLACEHOLDER` value
    unchanged ($3E8) so the rings.emp ensure still holds; name kept (no longer
    a placeholder — rename to `VRAM_RING` flagged).
  - DrawRings (rings.emp + twin): computes `d4 = VRAM_RING + Ring_Anim_Frame×4`
    once/call, SAT write uses d4 (was static immediate). Wires up the
    previously-dead Ring_Anim_Frame counter. RINGS len +0xA both shapes.
  - Re-pin: 28 pins (RINGS +0xA, downstream +0xA absorbed before the exception
    vectors; ASSEMBLED_LEN +0x1E0 = the +15-tile art). engine.inc (3 orgs),
    repin_pins.rs baselines, mixed_dac_rom.rs game_loop disp ($3A32/$4E60) all
    updated. Full strict 2142/0, clippy clean, repin --check no drift.
  - **Oracle-verified:** rings render as gold S3K rings (not the old blob);
    SAT tile = `$3E8 + Ring_Anim_Frame×4` (confirmed $3F0@frame2, $3F4@frame3);
    Ring_Anim_Frame cycles 0-3 on its own; VRAM decode confirms frame-0 = full-
    ring corner vs frame-2 = centred thin-edge (distinct art per frame). The
    marker correctly moved to $3F8.
  - FLAGGED: rename `VRAM_RING_PLACEHOLDER`→`VRAM_RING` (cross-file, byte-neutral).

## Correction (2026-07-12): ring colours were wrong — fixed per Volence review

The first cut of the art port had two colour bugs (shapes/layout were correct):
  1. **Wrong palette line.** DrawRings wrote the SAT attr as a bare tile index →
     CRAM line 0 (which in the OJZScroll boot = BGND_Palette, no gold). The donor
     convention (sonic_hack Ring.asm `vram_art(VRAM_Ring,1,1)`) is line 1 +
     priority — and aeon's OJZScroll loader copies `OJZ_Palette` → CRAM **line 1**
     (`Palette_Buffer+$20`), which carries the ring gold at idx 5/6/C/D.
  2. **Lossy hand-remap.** I'd used skdisasm's raw-index art and collapsed its
     4-shade scheme onto sonic.bin line 0 (5,6→E, 1→F, F→6) = flat orange, white
     where the outline belongs.
  Fix (verified the review's claims first: donor histogram = {5,6,C,D}; OJZScroll
  loads OJZ_Palette→line 1; Game_Entry = OJZScroll):
  - **Regenerated `ring_art.bin` from the DONOR** `sonic_hack/art/nemesis/Ring.bin`
    (already coloured for line-1 idx 5=outline/6=white/C=bright gold/D=dark gold),
    IDENTITY remap. Same 16-tile layout. compose_ring.py updated (REMAP={},
    donor-input docstring).
  - **DrawRings draws line 1 + priority**: d4 base = `RING_ART_ATTR =
    (1<<15)|(1<<13)|VRAM_RING_PLACEHOLDER` = $A3E8 (.emp const; twin spells the
    same arithmetic inline — the isolated SND-combo twin gate has no macros.asm,
    so `vram_art()` the macro can't be used there).
  - Byte-changing but SIZE-NEUTRAL (art still 16 tiles; attr immediate same 4-byte
    addi) → NO re-pin (repin --check clean). rings_port 5/5, strict 2142/0.
  - **Oracle-verified (spin cycling, not one frame):** rings render gold; SAT attr
    = $A3F0 = priority|line1|tile$3F0 at frame 2; full vs thin phases visibly
    distinct. Donor tiles $A-$D (collect-sparkle) not used yet — kept for later.

## Ledgered / owed
- oracle object-test harness fights static setup (re-drives rings + camera,
  forces debug-fly PSTATE_AIR) — R-A1 pixel-boundary + Bug-2 push live-repro
  need a normal level or a physics-freeze hook.
- controllers latch helper: blocked on cross-fragment label scope (F3 decline).
- `Radius` rename question: Volence's call.
