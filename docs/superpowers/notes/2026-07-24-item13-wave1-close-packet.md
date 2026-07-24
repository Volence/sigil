# CLOSE PACKET — item-13 wave-1 domain types (implementation)

**Branch:** `item13-wave1` (both repos). **Scope class:** BYTE-NEUTRAL.
**Canonical (moved by the transition-parcel merge, re-verified this parcel):**
plain `0bfa5b79`/421161 · debug `9d962703`/429204 — **UNCHANGED** (fresh dual
build at close). **Full paired strict:** `2499/0` (baseline 2490 + 9 pins).
**Clippy:** zero new (6 pre-existing warnings, none in the 4 changed test files).

Implements the FROZEN construct-walk #4 ratification (`7b9afa6`) per the impl
brief (`4dec6d3`). Rulings not re-litigated; the enforcement-reality finding
below was surfaced at the family-1 gate, not absorbed.

---

## What shipped, per family

**F1 · SongId / SfxId** (distinct u8) — `engine/system/types.emp` + the sound
seam. `Sound_PlayMusic(d0: SongId)`, `Sound_PlaySFX(d0: SfxId)`; the `SFXID_RING_*`
(sound_api.emp) and `SONG_*` (mt_bank.emp) const DEFS typed; construction sites
`as`-blessed (`Sound_PlayRing` moveqs, animate `.evt_sound` script-byte read).
68k-side only — the Z80 blob is untouched.

**F2 · AnimId / AnimFrame / MappingFrame** (three u8) — `FrameId` RENAMED →
`MappingFrame` (the drawn-frame identity: `mapping_frame`/`prev_frame`);
`anim_frame` retyped `u8` → `AnimFrame` (the script CURSOR); `AnimId` kept. The
tranche6 SST-drift probe needle updated for the rename.

**F3 · VramTile / VramAddr** (two u16) — `vram_art(tile: VramTile where 0..$1FFF)`
(the field-width bound PRESERVED across the retype); the tile-index consts
`VRAM_RING_PLACEHOLDER` / `VRAM_TEST_OBJ` born `VramTile`. `VramAddr` shipped as
the vocabulary contrast (its wave-1 users are ledgered — see below).

---

## Negative pins (RED-first) + positives

| Pin | Kind | Fires on |
|---|---|---|
| SfxId → SongId slot | unit (type_slice) | wrong newtype in a SongId param, found=SfxId |
| SongId → SfxId slot | unit | symmetric, found=SongId |
| `Sound_PlayRing` bless swap `as SfxId`→`as SongId` | **corpus** | Sound_PlaySFX d0/SfxId, found=SongId |
| AnimFrame → MappingFrame slot | unit | the highest-risk frame swap, found=AnimFrame |
| MappingFrame → AnimFrame slot | unit | symmetric |
| vram_art tile > $1FFF | unit (eval_guards) | `parameter tile: 8192 not in 0..8191` |

Positives: `retrofitted_corpus_has_zero_slot_mismatches` stays green (the F1
blesses satisfy every Sound_PlaySFX slot); `matching_frame_type_satisfies_slot`;
`vramtile_where_bound_passes_in_range`.

---

## HEADLINE (neither-bucket) — the enforcement-tier finding

The ratification's premise ("G5's checker exists for exactly this swap";
"every art_tile spelling checks for free") assumed enforcement the substrate does
NOT provide for two of three families. The G5 slice bites **only at register
call-slots**:

- **F1 has LIVE enforcement.** `Sound_PlayRing → Sound_PlaySFX` is an ungated
  register call-slot; the corpus swap pin fires. (The animate `.evt_sound` bless
  is correct for the SOUND_DRIVER_ENABLED=1 ROM but the corpus walk elides
  comptime-`if`-gated calls, so its coverage is the unit pins + the real ROM.)
- **F2 is DOCUMENTARY.** Every anim/frame value flows through SST **memory**
  (via `a0: *Sst`); no register call-slot carries one, and no field-store
  domain-check exists. The swap pins are synthetic — they prove the slice
  DISTINGUISHES AnimFrame/MappingFrame the moment such a value crosses a register
  param.
- **F3 is DOCUMENTARY.** Comptime-fn params are loosely bound (`check_arg_class`
  guards only Reg/Label); `vram_art(tile: VramTile)` + the typed consts carry
  meaning, not a check. The one surviving comptime check is vram_art's
  `where`-bound (pinned as a no-loss regression guard).

This is consistent with the existing `engine.types` module (Coord/Velocity/
AnimId are already largely documentary), and the frozen types were shipped as
ratified. The two substrate extensions that would give F2/F3 teeth — a
**field-store domain-check** (F2) and a **comptime-arg newtype-check** (F3) — are
ledgered as reopen markers, matching the pending-mechanism-marker / A4-i-gated
discipline.

---

## Watch-items — resolution

- **FlatIDXY preserves-verifier reopen** — **NOT triggered.** No typed REGISTER
  value is carried across a multiply through a conditional-save callee. The only
  arithmetic touches are (a) F2 `mapping_frame` → `add.w d2,d2` at animate.emp:284
  — but d2 is memory-LOADED (register untyped in the slice; the FIELD is typed,
  which creates no re-`as` obligation), and (b) F3 `vram_art`'s comptime
  shift/or — not the register-preserve-across-call pattern the verifier guards.
  Surfaced, not absorbed.
- **Optional-param Option A (`?`-marker) rider** — **LOG-AND-SPLIT** (exceeds the
  SIZE-CAP). `AnimateSprite (a0: *Sst, d3?: u8)` needs new grammar (`?`-param) +
  AST field + D1b `[call.input-undefined]` suppression across parser/ast/calls
  (sigil SRC — none of this parcel's open files, which are tests + aeon `.emp`),
  and is all-or-nothing: adding the param without the `?`-suppression would
  CREATE a D1b firing at `test_particle.emp:53`. Split to its own small
  byte-neutral parcel; Option D (typed duration byte) stays the enforcing
  successor. Ledgered.
- **Wave 2 (Tile/Block/Chunk, Coord/Velocity, S2-D8)** — untouched, A4-i-gated.

---

## Per-pass breakdown (port-loop)

**Step 3 — retrospect / language asks:**
1. **F2 field-store domain-check** — the enforcement path for memory-resident
   domain types (the biggest ask; makes the SST field typing bite).
2. **F3 comptime-arg newtype-check** — extend `check_arg_class` to reject a
   wrong-newtype / bare-int arg into a domain-typed comptime-fn param.
3. **Const-born provenance** — `SONG_*`/`SFXID_*` are typed at their DEFs, but a
   `moveq #CONST, dN` folds the immediate to a bare int (`CodeOperand::Imm`), so
   the const's type does not ride into the register; the live enforcement rides
   the explicit `as`-bless, not the const type. A const-born-typing mechanism
   (a typed-const immediate blesses its destination) would deliver the "near-zero
   ceremony" the ratification described. Ledger candidate.

**Step 5 — engine optimize:** none. Byte-neutral parcel; no perf work in scope,
canonical bytes are the acceptance floor.

**Neither-bucket headlines:**
- The **enforcement-tier finding** (above) — the run's top actionable.
- **`vram_bytes` does not exist** in aeon (stage-0): no tile→address call site,
  so it was ledgered not created (no dead scaffolding). `VRAM_PLANE_A/_B_BYTES`
  are byte ADDRESSES in address arithmetic (A4-i-gated), not VramTile — the
  ratification's "type VRAM_* as VramTile" needed per-const accuracy.
- **Process catch:** the shell cwd resets to the main checkout each Bash call, so
  bare `cargo` runs executed against master, not the worktree. Caught mid-F3;
  the authoritative `2499/0` was re-run cd'd into the worktree. (sigil changes are
  test-only, so byte gates — which test aeon `.emp` via the identical compiler —
  were valid regardless; only the pin tests needed the worktree cwd.)
- **Corpus walk elides comptime-`if`-gated calls** — the animate `.evt_sound`
  Sound_PlaySFX call (SOUND_DRIVER_ENABLED-gated) is not corpus-exercisable; a
  vacuous "sound-enabled" positive was removed rather than shipped misleading.

---

## Merge readiness

Byte gates green both shapes (canonical UNCHANGED); full paired strict `2499/0`;
negative pins RED-first; zero new clippy; ledger seed rows marked implemented +
reopen markers added. BYTE-NEUTRAL ⇒ at merge, CONFIRM canonical unchanged (not a
re-baseline); coordinate the sequential queue (this parcel is file-disjoint from
the transition parcel, already merged; the sprites-hardening parcel waits behind
this one — shared sprites.emp, though this parcel did not touch sprites.emp).

**Commits** — aeon: `5888d56` (F1) · `a46e953` (F2) · `137f881` (F3).
sigil: `9153a73` (F1 pins) · `cb6b949` (F2 pins + rename fixup) · `7f0b4c3`
(F3 bound pins) · this packet + ledger.
