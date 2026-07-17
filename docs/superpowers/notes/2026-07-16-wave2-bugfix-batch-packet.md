# Packet — wave-2 bugfix batch (B1/H-1/C1/D1/E1/P1a/P1b) → Fable merge gate

**Branch:** `fix/sprites-pb1-pb2` (both repos; continues the PB1/PB2 batch on the
same branch/worktrees). **Date:** 2026-07-16. Seven Fable-verified bugs + a
byte-neutral ledger rider. One commit per bug; per-bug verification; batch gate.

**Canonical provenance (branch tip):** plain `453087 / 824d4f2e`, debug
`461110 / b1f82f9a`. Full strict suite green (`SIGIL_STRICT_GATE=1`,
`AEON_DIR`=worktree); clippy delta zero (2 pre-existing warnings in
`struct_field_disp_plus_n.rs`).

---

## The fixes (one commit per bug)

### B1 — VSync_Wait torn-drain race (`vblank.asm`, aeon `32ffcb7`)
`VSync_Wait` cleared `VBlank_Flag` then set `VBlank_Ready` as two stores. An
IRQ6 in that window (Ready still 0) runs `VInt_Lag`, sets the flag → `.wait`
falls through with no real vsync while Ready is left =1 for the NEXT VBlank to
full-drain a mid-fill `Plane_Buffer` (the b96c861 hazard). **Fix: SR-mask the
pair** (`move.w sr,-(sp) / move.w #$2700,sr / … / move.w (sp)+,sr`) so it is
atomic. **Chose SR-mask over reorder:** reorder is 0-byte but costs a rare
1-frame stall and doesn't fix lag-count exactness; SR-mask is +8 B / ~34-40 cy
on a once-per-frame path, is strictly atomic (kills the tear AND the reorder's
stall), and makes `Lag_Frame_Count` exact. vblank.asm has no `.emp` twin.

### H-1 — Sound_PlayMusic repost race (`sound_api.{emp,asm}` + `z80_sound_driver.asm`, aeon `964458e`)
A repost while the Z80 is mid-`Snd_LoadSong` tore the live-read param block and
lost the trigger (Z80 cleared the slot at load END). **Fix (design note:
`aeon/docs/superpowers/2026-07-16-sound-repost-gate-design.md`):** 68k
`Sound_PlayMusic` spins until `MUSIC_SLOT==0` before posting (stopZ80 per
iteration — reliable read + Z80 runs between reads to clear); Z80 clears
`SND_REQ_MUSIC` at `.fm6_pan_owned` (right after the last param read) and the
end-clear is removed. **No snapshot** — the 68k gate already excludes reposts
while the slot is set, so the loader reads live and clears after (same guarantee,
none of the Z80-RAM/read-redirect risk). Z80 change is **byte-neutral** (`xor a`
placed where the Z flag is dead) so the driver blob doesn't shift the engine
block; only sound_api grows +0x22. Spin bound stated in the note; audit:
ping/fade/tempo/sample are single-byte/untearable, no gate needed.

### C1 — controller TH settling (`controllers.{emp,asm}`, aeon `11b9df0`)
One `nop` after each TH write; every reference (S1/S2/S3K, SGDK) uses two (mux
settling). Added the second nop at both TH sites. +4 B. Hardware-only —
**invisible to emulators by construction**, so verified against the references +
the ROM sentinel, not in oracle.

### D1 — dplc_layout merge overflow (`tools/dplc_layout.py`, aeon `c718bc1`)
`merge_adjacent_entries` could accumulate a run > 16 tiles; `write_dplc` masks
count to 4 bits → silent art corruption. **Fix:** re-split merged runs via the
existing `split_contiguous_entries` + a hard `assert 1<=count<=16` in
`write_dplc`. **Shipped-data check:** `--merge-only` is used nowhere in the
build, no `*_merged*` artifacts exist, dplc_layout isn't invoked by `build.sh` —
**the overflow never fired on shipped data; tool-only defense-in-depth.**
Self-tests 13/13.

### E1 + P1b — mega-act ceiling guards (byte-neutral, aeon `71e7ceb`)
E1: `Section_GetSecPtrXY`'s `adda.w` (flat×66) sign-extends → 496-section cap.
Added `ensure(MAX_ACT_SECTIONS*66 <= $7FFF)` at `act_descriptor.emp` (local
drift-guarded const → comptime, doesn't demand the extern when section compiles
standalone in the module-flip tests) + site comments in both section twins.
P1b: camera.asm/player_common.asm word-truncate `grid<<11` clamps above $FFFF px
(grid dim > 31); **found already guarded** (transitively, tighter) by
act_descriptor's `(GRID<<SHIFT) <= $8000` (grid ≤ 16). Added site comments at all
four clamp sites + broadened the existing assert's scope comment — no redundant
looser assert.

### P1a — parallax zero-deform flat-path (data-only, aeon `f44f534`)
`ojz_default`/`caves`/`locked_clouds` ran the full per-line H-deform sample loop
over an all-zero table (~computing 0 every line). `deformShiftDefault=15` marks
each band's channel "flat" (the runtime sentinel) → skips the loop; mode-select
keys on the non-NULL table pointer, so per-line HScroll mode ($03) and the
byte-identical HScroll output are retained. Data-only (parallax.asm has no `.emp`
twin; band-record shift bytes in place). rocking (real vDeform) / ojz_windy (real
sine) untouched.

---

## Scaffolding ripple (sigil) — accounted per fix

- **B1 (+8, hblank onward) + C1 (+4, controllers onward)** → +0xC through every
  downstream gated region. sigil `3244c5e`: repin (all engine bases +0xC,
  CONTROLLERS len +4, DRAW_SPRITE/DELETE_OBJECT etc.), engine.inc gate-resume orgs
  (scripted from the pins diff), `mixed_dac_rom.rs` (map bases + rom[] block
  windows + game_loop bsr PC-anchors + the controllers expected-bytes array now
  carries the 2 nops), `repin_pins.rs` hand-typed baseline.
- **H-1: Z80 byte-neutral → only sound_api +0x22.** sigil `e7b3099`: SOUND_API
  literal len 0x1E4→0x206 / 0x2DA→0x2FC (repin.toml override — no end-symbol,
  keeps the release ROM byte-clean), sound_api resume org, SOUND_{DRAIN_SFX_RING,
  PLAY_SFX,PLAY_RING}/SOUND_PLAY_SFX_OFF +0x22, repin_pins baseline.
- **E1/P1b/P1a/D1** added no ROM code bytes (comptime/comments/data-in-place/tool),
  so no pin shift.

## Ledger rider (sigil `7459732`, byte-neutral)
HBlank_Dispatch → RAM-jmp trampoline **RATIFIED** (Fable 2026-07-16) as t18
parallax step-0/1 first-consumer design — decided now while zero HInt handlers
exist, binding the dispatch contract. + the H-1 single-byte-slot gate pattern
recorded for future per-slot application.

---

## Live verification (oracle, DEBUG shape)

Reloaded ROM hash-matched the build; symbols from `s4.debug.lst`.

**Code sentinels (running ROM):**
- **B1** `VSync_Wait`: `40E7 46FC2700 …[clear+set]… 46DF` — SR save / mask $2700 /
  restore around the atomic pair, before `.wait`. ✓
- **C1** `.read_pad`: `10BC0040 4E71 4E71 1010` and `10BC0000 4E71 4E71 1210` —
  two nops after each TH write. ✓
- **H-1** `Sound_PlayMusic` opens with `33FC0100 00A11100` (stopZ80 request) +
  grant spin + `4A39` (tst.b MUSIC_SLOT) — the repost gate. ✓

**Boot clean:** 240 DEBUG frames, no assert halt (E1/P1b/H-1/D1 DEBUG asserts do
not fire spuriously); parallax renders.

**P1a A/B (profiler, same scene, master vs fixed):**
`Parallax_Update` **22631 cy (17.7%) → 8200 cy (6.4%) = ~14.4k cy/frame saved**
(matches the ~16k estimate); `GameState_OJZScroll_Update` 48.5% → 38.1%. Render
byte-identical (flat-path emits the same HScroll). The headline free-lunch win,
measured.

**Lag regression:** `Lag_Frame_Count` = **0** over 600 diagonal-drive frames
(fixed). No regression — and the profiler shows the fixed ROM is strictly cheaper
(P1a −14.4k dominates; B1/C1/H-1 add negligible per-frame cost; H-1's gate is
empty in the common no-load case).

### Honest caveats
- The B1 torn-drain and H-1 repost races are timing windows not reproducible on
  demand in this demo scene; the sentinels + code contract (+ design note) are the
  proof, plus the lag counter confirming B1's atomicity added no lag.
- C1 is hardware-only (mux settling) — no emulator models it; ROM sentinel + the
  S1/S2/S3K reference are the proof.
- P1a's reg-$0B/HScroll-DMA "still $03/896-byte" checks reduce to "HScroll output
  byte-identical", which the profiler A/B + identical render establish (the
  flat-path changes only *how* the same HScroll buffer is produced).

---

## Gate checklist for Fable
- [x] 7 bugs fixed, one commit per bug (E1+P1b bundled — one mega-act cluster,
      shared act_descriptor.emp; P1b's brief subsumes E1)
- [x] Byte gates re-pinned per byte-changing fix (B1/C1 +0xC; H-1 sound_api +0x22);
      engine.inc + mixed_dac_rom + repin_pins scaffolding updated; full strict green
- [x] Live: code sentinels (B1/C1/H-1), boot-clean, P1a profiler A/B (−14.4k cy),
      lag 0 on diagonal drive
- [x] H-1 design note written first (cross-seam protocol + bound + slot audit)
- [x] Ledger rider: HBlank trampoline RATIFIED
- [ ] **Fable gate → merge** (aeon + sigil pushed together, after the PB1/PB2 gate)

**aeon wave-2:** C1 `11b9df0`, B1 `32ffcb7`, E1+P1b `71e7ceb`, P1a `f44f534`,
D1 `c718bc1`, H-1 `964458e`.
**sigil wave-2:** repin/scaffolding `3244c5e`, sound_api repin `e7b3099`,
ledger `7459732`.
