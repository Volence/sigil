# Retro-fix batch 2 packet (retro-fix-audit-2)

Branch: `retro-fix-audit-2` (both repos, off master). **NOT merged — at Volence's gate.**
Closes the final five retro-audit sections (sound_api FULL sitting + t6/t4-adjacent/banks/twins).

**Gate status:** full strict suite **2211/0**, clippy clean, `repin --check` = plain untouched
(debug-shape only, as predicted). aeon builds both shapes clean (plain `s4.bin` 452512 B, debug
`s4.debug.bin` 460533 B). No forbidden files touched (ojz_scroll_test.asm / engine/ram.asm /
object_test_state.asm all untouched — the scene-pin/churn agents' turf).

Precondition verified at start: both repos on clean committed master; batch-1 (sigil `a17e0b7` /
aeon `5e946ca`) and t12 (`e2ad6d7`) merged.

---

## Byte-change summary (the ONLY ROM movement in the batch)

Everything is **release-byte-neutral**. The sole ROM change is the **debug shape** of
`sound_api` (findings 1+2's DEBUG asserts): debug `sound_api` grew **+0xF4** (0x1E4 → 0x2D8),
absorbed by the `org $10000` object-bank boundary so `EndOfRom` is UNCHANGED both shapes. Every
other item (findings 3-6, items 7-12) is comments / clobber-metadata / `ensure` / type-rename —
**zero ROM bytes**, confirmed by `s4.bin`/`s4.debug.bin` size-stability across the item-7-11 rebuild
and `repin --check` staying clean.

---

## Per-item outcomes

### sound_api.emp (+ twin sound_api.asm, lockstep) — the FULL sitting

**Finding 1 — Sound_PlayMusic song-id bounds assert (DEBUG).** Added two bare asserts after the
`andi.l #$FF, d0`: `assert.w d0, ne, #0` + `assert.w d0, ls, #SONG_COUNT`. A bad id indexed past
`SongTable[id-1]` and posted a garbage bank/window/patch block to the Z80 (streams as noise) — now
checked. **SONG_COUNT sourcing (the batch's "extern it; if no AS-side symbol exists, add one where
the truth lives"):** an AS-side symbol existed but was GATED — `SONG_COUNT` lived inside
`song_table.asm`'s `ifndef SIGIL_EMP_MT`, so the sigil mixed build (which gates that file out) left
it undefined for both the AS-twin (`#SONG_COUNT` at assemble time — imm16 can't defer, D2.27) AND
the `.emp` link-ref. **Fix: relocated `SONG_COUNT` to the ungated `config/sound_ids.asm`** beside
the SONG_* ids (which already moved there, T2 R2) — the campaign's "cross-seam symbols live ungated"
pattern (HBlank_Handler_Ptr / HW_PORT_1_DATA precedent). Byte-neutral (same value, same self-check
in song_table.asm). The assert comparand is a bare `#SONG_COUNT` on BOTH sides so the auto-message
bytes match (the verbatim-spelling rule; extern("...") would diverge).

**Finding 2 — Sound_PlaySFX ring-FULL drop assert (DEBUG).** Split the shared `.ps_drop` label:
the full-drop `beq` now targets a new `.ps_full` (its own landing), which in DEBUG fires
`raise_error "Sound_PlaySFX: SFX ring full (>7 in one frame)"` then falls into `.ps_drop`; the
dedup-skip still lands on `.ps_drop` (silent, correct). **Byte-neutral in release** — `.ps_full` is
an empty label aliasing `.ps_drop` when the `if DEBUG` block emits nothing. This is `raise_error`'s
**first `.emp` consumer** (shipped construct, path_swap.asm has the twin macro; diag_assert_vector.rs
covers the lowering). Twin lockstep: the DEBUG raise_error blob pushed two branches past `.s` range —
`beq .ps_ret` (top→end, spans the blob) needed `.w` in debug (the AS build errored loud on it); the
dedup `beq .ps_drop` (~84 B) still fits `.s`. The `.emp` bare branches auto-relax; the twin carries
the hand-set `ifdef __DEBUG__ beq.w / else beq.s` for `.ps_ret` only.

**Finding 3 — Sound_DrainSfxRing SR contract reworded (not restructured).** The `preserves(sr)` /
"SR restored" was half-true (the empty fast-path saves no SR — CCR clobbered). Reworded the header
to the precise contract: *interrupt mask never altered on either path; empty path leaves CCR
clobbered; posting path restores full SR; `preserves(sr)` marks the posting-path save/restore.*
**Judgment call: `preserves(sr)` KEPT** as the enforced-emphasis marker for the load-bearing
posting-path reliance (the DMA-window stopZ80 can't nest and corrupt the caller's mask); the
empty-path CCR clobber is now documented, not hidden, and is the S2-D7 lint's job to formalize.
Filed the S2-D7 CCR-liveness demand row (this = first instance).

**Finding 4 — Sound_PlayRing `clobbers(d0, a0)` → `clobbers(d0)`.** Body never touches a0; the
`jbra Sound_PlaySFX` tail-callee preserves it ENFORCED (`preserves(d1/a0)`). Updated RingCollision's
callee-list comment in rings.emp + rings.asm (`Sound_PlayRing d0/a0` → `Sound_PlayRing d0`) in
lockstep, same commit.

**Finding 5 — Sound_Init two load-bearing comment lines.** Documented (a) the bus is *released*
between probes (`startZ80` inside the loop) so the Z80 can actually run and boot — a continuous hold
would deadlock the handshake; (b) the no-timeout boot block is a chosen tradeoff (a dead sound
driver is a build/ROM fault, not a recoverable runtime state).

**Finding 6 / item 6 — stop_z80/start_z80 kill-list row.** Was MISSING. Added **row 24** to
twin-scaffolding-kill-list.md: the two comptime-fn templates mirror engine/macros.asm's
`stopZ80`/`startZ80` macros (invoked from sound_api.asm at PostByte/Init/PlayMusic/DrainSfxRing);
guard = the sound_api_port byte gates; kill = Spec 5 (dies with the AS twin). SoundId newtype +
S2-D7 CCR rows filed to the ledger (below).

### t6 objects

**Item 7 — TestSolid_Main `clobbers()` → `clobbers(d0-d3/a1)`.** Declared nothing while tail-calling
Draw_Sprite (the dangerous under-direction). Twin comment already correct.

**Item 8 — TestParticle + TestParticle_Main `clobbers(d0-d4/a1-a3)` → `clobbers(d0-d3/a1-a2)`.** The
TRUE callee union: ObjectMove(d0) ∪ AnimateSprite(d0-d2/a1-a2) ∪ Draw_Sprite(d0-d3/a1). The header
claimed to be the union — made it true. Twin comments updated in lockstep.

### Item 9 — the corpus-wide clobber declared-vs-union sweep (the batch's biggest item)

Every proc in the `.emp` corpus (~95 procs across 15 files) checked: declared `clobbers()` vs
body-writes ∪ transitive callee contracts (incl. tail-calls + `falls_into` targets + callee `out()`
regs). **Full table below.** Beyond the three known (items 4/7/8), the sweep found **two more
under-declarations** (both fixed) and **one deliberate over-declaration** (left as-is with its site
comment). The rest of the corpus is EXACT — a strong result for the hand-declared contracts; the
drift concentrates exactly where the deferred S2-D6 lint's transitive/`falls_into` reasoning beats
hand-declaration.

- **GameLoop** `clobbers(a0)` → **`clobbers(d0-d7/a0-a6)`** (UNDER, fixed). The fixed callees
  (VSync_Wait→d0; Sound_DrainSfxRing→d0/d1/a0) already trash d0/d1; the `jsr (a0)` state dispatch
  runs arbitrary game code. GameLoop never returns (`jbra GameLoop`), so the contract is nominal —
  but the signature must not falsely claim to preserve registers the body destroys. Widened to the
  honest full set + a comment naming the dispatch + noreturn. **(Decision for Volence's eye: the
  alternative was narrow-clobbers + a noreturn-exemption comment; I chose the honest exhaustive set
  per the batch's "fix under-declarations" rule.)**
- **TestSolid_Init** `clobbers()` → **`clobbers(d0-d3/a1)`** (UNDER, fixed). `falls_into
  TestSolid_Main` (Draw_Sprite's set) — a caller sees Main's clobbers, exactly as TestParticle
  declares its fallen-into set. Twin comment updated.
- **Collision_GetType** `clobbers(d1,d2,d3,a0) out(d0)` — over-declares d3 (the body only READS d3,
  passing it to Tile_Cache_GetCollision). **Left AS-IS** — deliberate sensor-register convention,
  documented at the module header + the proc header ("set d3 before EVERY call; not preserved by
  contract"). Per the batch rule (over-declarations tighten unless deliberate — then a site comment
  says so), the site comment already says so. No change.

### Item 10 — data-bank hand-synced mirrors closed

**mt_bank.emp:** the UNCHECKED `SONG_*` mirrors now drift-guard via `ensure(extern(...))` against
config/sound_ids.asm — `SONG_MOVINGTRUCKS` + `SONG_COUNT` (both ungated after finding 1's
relocation, so they resolve at link in ALL shapes; SONG_COUNT is the actually-consumed value).
`SONG_DRUMTEST`/`SONG_HCZ2` stay documentation-only (DEBUG-gated in sound_ids.asm → extern would be
unresolved in the plain shape; not consumed; a renumber is caught by the TABLE ORDER regardless).
Prose updated ("SEAM CLOSED"). **sfx_bank.emp:** the derived `SFX_ID_BASE`/`SFX_COUNT`/`SFX_TABLE_LEN`
mirrors now also `ensure(extern(...))` back to sound_ids.asm (all three ungated) — closes the
"stays hand-synced" seam (prose updated). All 5 isolated bank tests (mt_port, mt_negative_probes,
mt_dual_carrier, sfx_port, sfx_negative_probes) + mixed_dac_rom updated: synthetic SONG_*/SFX_* equs
supplied where the standalone link needs them; guard counts bumped (mt 5→7, sfx 1→4).

### Item 11 — Radius → HitboxDim (Volence-approved rename)

types.emp `newtype Radius` → `HitboxDim` (+ doc rewrite, NAME MISMATCH flag dropped — resolved);
sst.emp's import + `width_pixels`/`height_pixels` annotations renamed. entity_window.emp:450's
"Radius ±1" is an unrelated spatial-radius COMMENT — left. **Byte-neutral by construction (erasing
type):** confirmed — the aeon rebuild produced identical `s4.bin`/`s4.debug.bin` sizes and
`repin --check` stayed clean (types erase; the .asm twins reference SST_ offsets, not the type name).

### Item 12 — ledger jots (all appended to campaign-gap-ledger.md)

1. **SoundId newtype** demand instance (newtype-candidates class).
2. **S2-D6 checked-clobbers** demand += 5 confirmed drift instances (the item-9 sweep hits).
3. **S2-D7 CCR-liveness** first concrete demand instance (DrainSfxRing SR half-truth, finding 3).

---

## Item 9 — full sweep table (every proc named)

Verdict key: `exact` / `over-declares X` / `under-declares X` / deliberate. Callee dictionary
resolved from source: VSync_Wait→d0, Debug_MusicToggle→d0-d2/a0-a1, Tile_Cache_GetCollision→d0-d2/a0
(reads d3), QueueDMA_Important/Deferrable→d0-d4/a1-a2.

| File | Proc | Declared | Verdict |
|---|---|---|---|
| vdp_init.emp | VDP_Shadow_Init | d0/a0/a1 | exact |
| vdp_init.emp | Flush_VDP_Shadow | d0-d3/a0-a1 | exact |
| math.emp | GetSineCosine | () out(d0,d1) | exact |
| game_loop.emp | **GameLoop** | ~~a0~~ → d0-d7/a0-a6 | **under-declared (FIXED)** |
| game_loop.emp | GameState_Idle | () | exact |
| hblank.emp | HBlank_Dispatch | preserves(d0-d1/a0) | exact |
| hblank.emp | HBlank_Null | () | exact |
| controllers.emp | Read_Controllers | d0,d1,a0 | exact |
| collision_lookup.emp | Collision_GetType | d1,d2,d3,a0 out(d0) | **over-declares d3 — DELIBERATE (sensor convention, documented); no change** |
| rings.emp | RingBuffer_Add | d4,a0 | exact |
| rings.emp | RingBuffer_Remove | d1,d2,a0,a1 | exact |
| rings.emp | RingBuffer_Clear | () | exact |
| rings.emp | DrawRings | d0-d4/d6-d7/a0 out(d5) | exact |
| rings.emp | RingCollision | d0-d7/a0-a3 | exact |
| sound_api.emp | Sound_PostByte | () preserves(sr) | exact |
| sound_api.emp | Sound_Init | () preserves(sr) | exact |
| sound_api.emp | Sound_Ping | a0 | exact |
| sound_api.emp | Sound_PlaySample | a0 | exact |
| sound_api.emp | Sound_PlayMusic | d0-d4/a0-a1 preserves(sr) | exact |
| sound_api.emp | Sound_PlaySFX | preserves(d1/a0) [clobbers d0] | exact |
| sound_api.emp | Sound_DrainSfxRing | d0,d1,a0 preserves(sr) | exact (SR prose reworded — finding 3) |
| sound_api.emp | **Sound_PlayRing** | ~~d0,a0~~ → d0 | **over-declared a0 (FIXED — item 4)** |
| sound_api.emp | Sound_StopMusic | d0,a0 | exact |
| sound_api.emp | Sound_SetTempo | a0 | exact |
| sound_api.emp | Sound_FadeOut | d0,a0 | exact |
| sound_api.emp | Sound_FadeIn | d0,a0 | exact |
| collision.emp | TouchResponse | d0-d7/a0-a3 | exact |
| collision.emp | Touch_HandlerTable | (dispatch table) | n/a — not a clobber-bearing proc |
| collision.emp | Touch_None…Touch_Touch | (falls_into chain) | exact (each aliases the single rts) |
| collision.emp | Touch_Hurt | () | exact |
| collision.emp | Touch_Solid | d0-d5 | exact |
| animate.emp | AnimateSprite | d0,d1,d2,a1,a2 | exact (callback jsr + RefreshSpritePieceCount + DeleteObject ⊆) |
| animate.emp | RefreshSpritePieceCount | d2,a1 | exact |
| core.emp | InitObjectRAM | d0,d1,a0,a1 | exact |
| core.emp | AllocDynamic | d0 out(a1) | exact |
| core.emp | AllocEffect | d0 out(a1) | exact |
| core.emp | DeleteObject | d0,a1 | exact (d1 movem-saved around the scan) |
| core.emp | CompactDynamicLive | d0,d1,a0,a1,a2 | exact |
| core.emp | DrainDynamicPending | d0,d1,d2,a0,a1 | exact |
| core.emp | RunObjects | d0-d6/a0-a6 | exact |
| core.emp | Debug_AssertObjLoop | () | exact |
| core.emp | RunObjects_Frozen | d0-d6/a0-a6 | exact |
| core.emp | ObjectMove / ObjectMoveX / ObjectMoveY | d0 | exact |
| dplc.emp | Perform_DPLC | d0-d4/a1-a2 | exact (a3 movem-saved around the enqueue) |
| dplc.emp | Perform_DPLC_Deferrable | d0-d4/a1-a2 | exact |
| sprites.emp | InitSpriteSystem | d0,a0 | exact |
| sprites.emp | Draw_Sprite | d0,d1,d2,d3,a1 | exact |
| sprites.emp | Render_Sprites | d0-d7/a0-a6 | exact |
| sprites.emp | Emit_ObjectPieces | d0,d1,d4,a0,a1,a3,a6 out(d5) | exact |
| sprites.emp | InsertSpriteMasks | d0,d1 out(d5) | exact |
| entity_window.emp | (all 30 procs) | (as declared) | exact |
| test_solid.emp | **TestSolid_Init** | ~~()~~ → d0-d3/a1 | **under-declared (FIXED — falls_into)** |
| test_solid.emp | **TestSolid_Main** | ~~()~~ → d0-d3/a1 | **under-declared (FIXED — item 7)** |
| test_particle.emp | **TestParticle** | ~~d0-d4/a1-a3~~ → d0-d3/a1-a2 | **over-declared (FIXED — item 8)** |
| test_particle.emp | **TestParticle_Main** | ~~d0-d4/a1-a3~~ → d0-d3/a1-a2 | **over-declared (FIXED — item 8)** |

(entity_window's 30 procs all verified exact — the tranche-12 port declared them carefully; the
sweep transcript names each individually.)

---

## Re-pin diff (debug-shape only — `repin --check` = plain untouched)

Regenerated `pins.rs` from the rebuilt reference ROMs:

- `SOUND_API`: `len` region converted from a literal (`len = 0x1E4`, no end symbol) to an **end
  symbol** `Sound_Api_End` (a zero-byte anchor added to sound_api.asm at the resume position) so
  repin derives PER-SHAPE lengths. `plain_len` 0x1E4 (unchanged); `debug_len` **0x1E4 → 0x2D8**.
- `SOUND_DRAIN_SFX_RING` debug 0x7812 → **0x7908** (+0xF6); plain unchanged.
- `SOUND_PLAY_RING` debug 0x7862 → **0x7958** (+0xF6); plain unchanged.
- `SOUND_PLAY_SFX` debug 0x77CC → **0x787C** (+0xB0); plain unchanged.
- `SOUND_PLAY_SFX_OFF`: `usize 0x100` → **`ShapeOffset { plain: 0x100, debug: 0x1B0 }`** (per-shape;
  PlayMusic's two asserts precede Sound_PlaySFX in the debug region). repin.toml offset gained
  `per_shape = true`.
- `engine.inc` SIGIL_EMP_SOUND_API debug resume org **$78B0 → $79A4** (= debug_base + new debug_len;
  plain $5F94 unchanged). This is the row-6/row-mixed-gate hand-maintained pin the ledger's
  2026-07-13 "mixed-gate maintenance" row flags — batch 2 is another instance of that burden (the
  sound_api debug org + the mixed_dac_rom sound_api `size` both hand-updated).
- `repin.toml`: sound_api region `len` → `end = "Sound_Api_End"`; SOUND_PLAY_SFX_OFF per_shape.
- `repin_pins.rs` baseline: SOUND_API.debug_len 0x2D8, SOUND_PLAY_SFX_OFF ShapeOffset.

Test-harness updates (all to keep gates green with the new DEBUG shape): sound_api_port grew a
`debug` bool + per-shape `len`/`play_sfx_off` + DEBUG-per-shape define + the SONG_COUNT synthetic equ
+ the two MDDBG__ErrorHandler debug-only labels (the raise_error blob's tail). mixed_dac_rom passes
DEBUG to the 5 sound_api.emp compiles + per-shape sound_api map size. tranche5_negative_probes
lowers sound_api with DEBUG=0.

---

## Ledger rows closed / added

**Added (item 12):** SoundId-newtype demand; S2-D6 checked-clobbers demand += 5; S2-D7 CCR-liveness
first instance (all `[retro-fix-batch-2, 2026-07-13]`).

**Kill-list:** row 24 (stop_z80/start_z80 comptime-fn templates ↔ macros.asm).

---

## Notable engineering notes (neither-bucket)

- **`build.sh` continues past AS errors silently.** The first debug build hit
  `sound_api.asm(158): error #1370: jump distance too big` (the pre-revert `.ps_drop`=.w overshoot),
  logged to s4.log, but build.sh proceeded to p2bin and produced a MALFORMED ROM (sound_api region
  zeroed) that looked plausible by size. Cost real debugging time chasing a "listing says code here,
  ROM has zeros" paradox. Lesson recorded in the workflow: **always `grep -c error s4.log` after
  every aeon build** — done for the rest of the batch.
- **`raise_error` first `.emp` consumer** — the ring-full drop is the corpus's first use (assert had
  shipped; raise_error was spec'd + tested but unused). Byte-parity held through the diag lowering.
- **SONG_COUNT cross-seam relocation** — the imm16-in-a-cross-seam-comparand hit the unshipped
  imm16-deferral gap (kill-list row 10) in the mixed partial-gate combos; the campaign's
  "cross-seam symbols live ungated" pattern (move to sound_ids.asm) is the clean resolution, no
  language change needed.

---

## Recommendation

Mergeable pending Volence's gate. The two decision-flagged calls to eyeball: (1) GameLoop widened to
the full clobber set vs a narrow+noreturn-comment (item 9); (2) Sound_DrainSfxRing keeping
`preserves(sr)` as posting-path emphasis (finding 3). Both documented in-file. Do not merge without
the gate.
