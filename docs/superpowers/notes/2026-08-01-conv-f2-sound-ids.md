# 2026-08-01 — CONV-F2 #24: the sound-ids flip (close packet)

Status: DONE. Merge state lives in the campaign log, not here. Branch
pair `conv-f2-sound-ids` (aeon + sigil, both off the merged Parcel-F tips). Parcel F's
#24 (`config/sound_ids.asm`) is **FLIPPED and the file DELETED**; the SFX-bank-count
family is **DISSOLVED into `sfx_bank.emp`** (the ruling's mechanical path). Six targets
byte-identical + all 15 Z80 blobs identical; strict 2867 → 2866 (−1 retired probe).

## §0 — THE HEADLINE

`games/sonic4/config/sound_ids.asm` is **DELETED**. Two moves:

1. **The clean subset → a new `.emp` authority.** The song ids (`SONG_*`), symbolic SFX
   ids (`SFXID_*`), the spindash-rev special case (`SFXID_REV_LOOP`, moved from
   `config/game.asm`), and the SFX priority ladder (`SFXPRI_*` + its 7-bit guard) move to
   **`games/sonic4/config/sound_ids.emp`** (module `games.sonic4.sound_ids`) — a direct
   clone of #21's `games.sonic4.constants` pattern. Game-side consumers `use` it; the
   game-agnostic engine + residual AS read it via the harvest (`harvest_game_constants`,
   with a new `DEBUG` seed for `SONG_COUNT`).
2. **The SFX-bank counts → their derived authority.** `SFX_ID_BASE` / `SFX_COUNT` /
   `SFX_TABLE_LEN` dissolve into **`sfx_bank.emp`** (the module that owns the bank they
   count, where they were ALREADY `SfxTable.min_key`/`.count`/`.len`). The ruling's
   mechanical path — proven, executed, quadruple mirror collapsed to one derivation.

## §1 — THE FLIP (aeon)

- **NEW `games/sonic4/config/sound_ids.emp`** (`module games.sonic4.sound_ids`): `pub const`
  for `SONG_MOVINGTRUCKS`/`DRUMTEST`/`HCZ2` (untyped ids; DRUMTEST/HCZ2 ungated — the
  DEBUG gating is on their USE, not the value), `SONG_COUNT = if DEBUG == 1 { 3 } else { 1 }`
  + its `< $FF` ensure, the 9 `SFXID_*` untyped ids, `SFXID_REV_LOOP = SFXID_SPINDASH`, the
  8 `SFXPRI_*` tiers + the 7-bit `ensure(((… | …) & $80) == 0, …)` guard (the AS
  `if/fatal` transliterated).
- `config/sound_ids.asm`: **DELETED** (both moves complete). `gameConfigIncludes`
  (main.asm) + `m1c_root.asm` drop the include.
- `config/game.asm`: the `SFXID_REV_LOOP = SFXID_SPINDASH` line REMOVED (→ sound_ids.emp);
  the stale `SFXID_RING_*`/`SFX_ID_BASE`-family comments refreshed to name the new homes.
- **Game-side consumers** (mirror + guard → `use games.sonic4.sound_ids`): `player_common`
  (SFXID_SKID), `player_ground` (SFXID_ROLL/JUMP/SPINDASH), `player_spindash`
  (SFXID_SPINDASH/DASH), `game_debug` (SONG_* ×3 + SFXID_* ×8, mirror consts + 11 ensures
  DELETED).
- **Engine-side (KEPT with extern guards, resolved via the harvest)**: `sound_api.emp`'s
  TYPED `SFXID_RING_RIGHT/LEFT: SfxId` mirrors + ensures stay (engine is game-agnostic,
  cannot `use` a game module; the `SfxId` newtype is the pre-ruled deferral). `mt_bank.emp`
  keeps its TYPED `SONG_*: SongId` mirrors + `SONG_MOVINGTRUCKS` ensure AND a LOCAL
  `SONG_COUNT` (the focused seam-2 MT emit lowers mt_bank without a game module in scope,
  so SONG_COUNT must resolve comptime-locally to size SongTable/SongPatchTable), both
  drift-guarded against the authority. `boot.emp`'s `moveq #SONG_MOVINGTRUCKS` resolves via
  the harvest EquSym.

## §2 — THE SFX_ID_BASE DISSOLUTION — outcome under the ruling

**The ruling's gate was tested empirically and PASSED.** A throwaway probe ran
`eval_all_pub_consts(sfx_bank.emp)` with only `DEBUG=0` seeded: it resolved
`SFX_ID_BASE=$33`, `SFX_COUNT=9`, `SFX_TABLE_LEN=135` with **zero parse + zero eval
errors**, standalone — `SfxTable.min_key`/`.count`/`.len` are table-shape metadata that
resolve WITHOUT the bank emit. No circularity between the count and the bank emit (the
eval is a separate, lighter path — the exact `sound_authority_consts` precedent). So the
dissolution is **mechanical**; per the ruling, done in this parcel.

The quadruple mirror — (a) `sound_ids.asm`'s 3 hand equs, (b) seam1's `seam_emit_config`
`SFX_ID_BASE=0x33`/`SFX_TABLE_LEN=0x87`, (c) seam2's co-link carrier `… = 135`, (d)
`sfx_bank.emp`'s derivation — collapses to (d) alone:

- **sfx_bank.emp**: the 3 consts made `pub`; their 3 `ensure(extern("SFX_*") == …)` drift
  guards RETIRED (self-referential once sfx_bank is the sole authority — the derivation IS
  the truth, nothing external to cross-check).
- **native.rs**: new `GameProfile.game_sfx_bank_rel`; `assemble_as_side` harvests the 3
  counts as guarded `-D` + link EquSyms so the residual AS `soundBankHead`
  (sound_bank.inc's `SfxBlobWinTab` span guard) reads `SFX_TABLE_LEN`.
- **seam1.rs**: new `sfx_bank_authority_consts` (memoized eval, the sfx_bank sibling of
  `sound_authority_consts`); `resolve_consts` threads it between the sound authority and
  `seam_emit_config`, so the resident SFX reader's `SFX_ID_BASE`/`SFX_TABLE_LEN` `-D` flow
  from the derivation. The `seam_emit_config` hardcodes DELETED.
- **seam2.rs**: the SFX head co-link carrier sources `SFX_TABLE_LEN` from that authority
  (the hardcoded `135` DELETED); the `SFX_ID_BASE`/`SFX_COUNT` carriers gone (their
  consuming ensures retired).
- **Surviving consumer guard**: `sfx_blob_win_tab.emp`'s `ensure(135 ==
  extern("SFX_TABLE_LEN"))` — its hand 135-cell body vs the derived length; genuine, extern
  resolves against the authority via the co-link carrier / harvest.

`SFXID_REV_LOOP` and `SFX_BLOB_BANK` stay in `seam_emit_config` (game config the resident
emit needs; NOT the SFX_ID_BASE family the ruling scoped) — `SFXID_REV_LOOP`'s hardcode is
now a mirror of `sound_ids.emp`, gap-ledgered.

## §3 — BYTE-IDENTITY PROOF (all six + 15 blobs)

| target | built | golden | match |
|---|---|---|---|
| s4          | `6cf74e65` / 412127 | `6cf74e65` / 412127 | ✓ build.sh |
| s4.debug    | `16615e46` / 421958 | `16615e46` / 421958 | ✓ DEBUG=1 build.sh |
| demo / demo.debug / config_a / config_b | — | — | ✓ via strict `native_offcanonical_full` (config_a/b `_full_file`, demo `_anchor_matches_golden`) |

**15 Z80 blobs**: `emit_sound_blob` re-run after each step, md5-diffed against the pre-change
baseline — **0 drift** at every checkpoint (incl. after the seam1/seam2 authority-sourcing
change, the sensitive step). The 15: `z80_sound_blob{,_debug}`, `dac_sample_tab`,
`dac_blip_bank`, `dac_shared_bank`, `mt_bank{,_debug}`, `sfx_bank{,_debug}`,
`sfx_blob_win_tab{,_debug}`, `seq_opcode_tab{,_debug}`, `sound_tables_z80`,
`movingtrucks_pitchtable`.

## §4 — RETIREMENTS + RE-HOMES (strict 2867 → 2866, net −1)

| retired test | file | re-homed to |
|---|---|---|
| `doctored_extern_fires_drift_guard` | `game_debug_port.rs` | game_debug's SFXID/SONG mirror consts + `ensure(extern)` guards are GONE (game-side `use` has no mirror to drift); re-homed to the config_a whole-ROM byte-identity golden + the SURVIVING engine-side authority guards (sound_api's `SFXID_RING_*` + mt_bank's `SONG_MOVINGTRUCKS`/`SONG_COUNT` ensures, resolved via the harvest — `sfx_negative_probes`/`sound_migration` cover their liveness) |

Count-neutral test edits: `game_debug_port` (song/SFX ids → comptime defines; button/addr
carriers only); `test_p1_player_port::p1_drift_guards_all_pass` (`>= 1` → `== 0`,
player_common's last mirror guard SFXID_SKID retired); `sfx_port` + `sfx_negative_probes`
(id-count carriers dropped, sfx_bank ensure-count `4 → 1`). Net: **2866 / 0 / 4.**

## §5 — STEP-3 (retrospect) vs STEP-5 (engine optimization)

- **Step-3:** ~21 mirror consts + ~22 `ensure(extern)` drift guards removed across 4
  game-side `.emp` files, replaced by `use`. The SFX_ID_BASE quadruple mirror collapsed to
  the single `sfx_bank.emp` derivation (the 3 self-referential ensures + the seam1/seam2
  hardcodes retired). `config/sound_ids.asm` fully deleted — one more residual AS config
  file gone. Kill rows 54 (game_debug songs/sfx), 76/81 (player SFXID halves) CLOSED; a new
  row for the SFX-bank dissolution.
- **Step-5:** none. Pure ownership + authority-sourcing move; no lowering changed, no ROM
  byte moved. `sfx_bank_authority_consts` is memoized per aeon root (one eval, not per-emit).

## §6 — NEITHER-BUCKET HEADLINES

- **The eval-on-a-data-module gate.** The ruling's "IF it entangles (circularity between the
  bank emit and the count), STOP" was a live risk — `SFX_ID_BASE` derives from a `table`
  with 18 `embed()`s. The probe settled it: `eval_all_pub_consts` resolves the table
  METADATA (min_key/count/len) without emitting payload or reading blob bytes, so the
  resident-blob emit can source the count from a separate lighter eval with no circularity.
  This is the reusable finding: a game DATA module's derived counts CAN be a harvest/seam
  authority.
- **The typed newtype layer stays split, deliberately.** `SfxId`/`SongId` are the pre-ruled
  language-round deferral, so the UNTYPED id values live in the authority and the TYPED
  `: SfxId`/`: SongId` mirrors stay local to the two engine-side consumers that can't `use`
  a game module (sound_api, mt_bank) — each extern-guarded. Consistent with #21's
  `sonic_anims`/`camera` extern-guard survivors.
- **mt_bank's local SONG_COUNT is not a mirror to eliminate — it's an emit-context
  constraint.** The seam-2 MT bank lowers mt_bank.emp in isolation (no game module in
  scope), so SONG_COUNT must resolve comptime-locally there; the ensure(extern) ties it to
  the authority. Same shape as the engine-side typed mirrors.

## §7 — KILL-ROW CLOSURES + GAP-LEDGER

Kill-list (same-commit): row **54** (game_debug songs/sfx) CLOSED (→ `use`); rows **76/81**
SFXID halves CLOSED (player_common SFXID_SKID; player_ground/spindash SFXID_*); NEW row for
the SFX_ID_BASE quadruple-mirror dissolution. Gap-ledger: `SFXID_REV_LOOP` still hardcoded
in `seam_emit_config` (a mirror of sound_ids.emp; sourcing it from a sound_ids authority-
eval is the nicety); stale `config/sound_ids.asm` "truth" comment references in untouched
files (fix-at-next-touch).

## §8 — FILE MANIFEST (branches unmerged)

**aeon (`conv-f2-sound-ids`):** commits `e6b2f8a` (clean subset) + `bdec8c4` (SFX_ID_BASE
dissolution). NEW `config/sound_ids.emp`; DELETED `config/sound_ids.asm`; M `config/game.asm`,
`main.asm`, `engine/sound/sound_api.emp`, `data/sound/mt_bank.emp`, `debug/game_debug.emp`,
`player/player_common.emp`, `player/player_ground.emp`, `player/player_spindash.emp`,
`data/sound/sfx/sfx_bank.emp`, `data/sound/sfx_blob_win_tab.emp`.

**sigil (`conv-f2-sound-ids`):** commits `38eb547b` (harvest + port ripple) + `7e723799`
(seam/harvest authority sourcing). M `native.rs` (game_sound_ids_rel + game_sfx_bank_rel +
harvest DEBUG seed + wiring), `seam1.rs` (sfx_bank_authority_consts + resolve_consts +
seam_emit_config trim), `seam2.rs` (carrier from authority), `m1c_root.asm`,
`m1c_vector_table.rs`, `game_debug_port.rs`, `test_p1_player_port.rs`, `sfx_port.rs`,
`sfx_negative_probes.rs`. Plus this note + census/kill-list/gap-ledger.
