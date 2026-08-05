# 2026-08-01 — SOUND-E2: the sound-constants mirror dissolution (close packet)

Status: DONE. Merge state lives in the campaign log, not here. Branch pair
`sound-e2-mirror` (aeon + sigil, both off the E1 branch). The seam-1 399-entry
resident-blob const mirror and seam-2's pinned DAC-head carriers are **DISSOLVED** —
every resident-module `-D` value now flows from `engine/sound/sound_constants.emp`
(the E1 authority) through the shared `eval_all_pub_consts` path. **All 15 generated
Z80 blobs byte-identical (before→after, every step); all six ROM targets
byte-identical to the chain tips; strict 2868/0/4.** `dac_sample_tab.emp`'s 2
externs swapped to `use` (the E1 deferral). Spec: §2 E2 of
`docs/superpowers/specs/2026-08-01-sound-constants-flip-design.md` (now stamped
SHIPPED). This is the payoff parcel: **no hand-maintained copy of any engine sound
constant survives.**

## §0 — HEADLINE

E1 created the authority; E2 makes it the *only* source. `crates/sigil-harness/src/seam1.rs`
kept five hand-maintained `(name, value)` tables — `driver_consts` 125 /
`sequencer_consts` 112 / `sfx_consts` 75 / `fm_consts` 57 / `psg_consts` 30 =
**399 values** — fed as comptime `-D` to each resident Z80 module's
`emit_sound_blob` lower, and `seam2.rs` pinned `DAC_SAMPLE_COUNT`="10" /
`DacSample_len`="9" equ carriers (its self-described "stand-in for
`sound_constants.asm`"). Both hand-copied, cross-checked by nothing. Now seam1
keeps only the NAME lists; every value resolves from `sound_constants.emp` via
`sound_authority_consts` (a memoized `eval_all_pub_consts`). seam2 seeds the two
DAC sizes as comptime `-D` from the same authority; `dac_sample_tab.emp`'s guard
`use`s them and folds at comptime; the carriers are deleted.

## §1 — THE AUTHORITY-FLOW DIAGRAM (before 3 copies → after 1)

```
BEFORE E1 (the triple mirror, cross-checked by NOTHING):
   engine/sound_constants.asm  (AS, 1481 ln)   ── the nominal "single source"
   seam1.rs 5 tables (399 values)              ── resident-blob -D, hand-copied
   seam2.rs DAC carriers ("10"/"9")            ── co-link stand-in, hand-copied
   + 33 68k-.emp link-externs

AFTER E1 (authority born; mirrors still hand-maintained):
   engine/sound/sound_constants.emp  ★ SOLE AUTHOR of the contract
   seam1.rs 5 tables (399 values)    ── STILL hardcoded (E1 kept)
   seam2.rs DAC carriers             ── STILL pinned  (E1 kept)
   68k consumers → use engine.sound_constants

AFTER E2 (this parcel — one authority, drift structurally impossible):
   engine/sound/sound_constants.emp  ★ SOLE AUTHOR
        │  eval_all_pub_consts (the shared build eval)
        ├──> seam1.rs  resolve_consts(names)   ── 384 values, ZERO hand-maintained
        └──> seam2.rs  -D DAC_SAMPLE_COUNT/DacSample_len  ── carriers deleted
   dac_sample_tab.emp: extern("…") → use engine.sound_constants.{…}
   (14 non-authority names → seam_emit_config, each with provenance; see §3)
```

## §2 — THE BLOB-IDENTITY PROOF (all 15, before→after, every step)

Baseline snapshot taken pre-change; `emit_sound_blob --aeon . --out-dir
engine/sound/generated` re-run after **each** step; all 15 `.bin` md5-identical at
every checkpoint.

| step | change | 15-blob diff |
|---|---|---|
| baseline | pre-E2 tips | (snapshot) |
| seam1 name-lists + `resolve_consts` | 399 values sourced from authority | **0 drift** |
| + `FmPatch_fp_tl` added to authority | struct-offset alias homed | **0 drift** |
| + seam2 DAC carriers → authority `-D`, `dac_sample_tab` extern→use | carriers deleted | **0 drift** (incl. `dac_sample_tab.bin`, the DAC head) |

The 15: `z80_sound_blob{,_debug}.bin` (resident driver); `dac_sample_tab.bin`,
`dac_blip_bank.bin`, `dac_shared_bank.bin`, `mt_bank{,_debug}.bin`,
`sfx_bank{,_debug}.bin`, `sfx_blob_win_tab{,_debug}.bin`,
`seq_opcode_tab{,_debug}.bin`, `sound_tables_z80.bin`,
`movingtrucks_pitchtable.bin`.

### Six ROM targets (all == chain tips)

| target | built | tip |
|---|---|---|
| s4 | `6cf74e65` / 412127 | `6cf74e65` / 412127 ✓ (build.sh) |
| s4.debug | `16615e46` / 421958 | `16615e46` / 421958 ✓ (DEBUG=1 build.sh) |
| demo / demo.debug / config_a / config_b | — | ✓ via `native_offcanonical_full` (7/0), == `9bb8c993` / `bc7678d0` / `78df5e6a` / `f38f609b` |

## §3 — SEAM1-ONLY VALUE FINDINGS (the 15 names not in the authority)

The prototype diff (authority pub-consts, `Z80_RAM=$A00000` seeded) resolved **332
consts, 0 errors, 0 mismatches** against all five tables — 384 of the 399 names hit
the authority exactly. 15 names were authority-absent, classified by provenance:

**Homed in the authority (belongs there):**
- `FmPatch_fp_tl` (6) — a `FmPatch` struct offset. The authority owns `FmPatch`
  and every other struct-offset alias; E1 simply hadn't authored this one. Added as
  `pub const FmPatch_fp_tl = offsetof(FmPatch, fp_tl)` (byte-neutral — no 68k glob
  consumer defines it; the resident `sound_fm.emp` reads it via `-D`, unchanged).

**Genuinely emit-tool / game config → `seam_emit_config`, each with provenance:**
- `DacSampleTable` — the DAC descriptor head's `$8000`-window VMA. **DERIVED** from
  seam-2's one `DAC_SAMPLE_TAB_LMA` (`0x8000 + (DAC_SAMPLE_TAB_LMA -
  SOUND_TABLES_Z80_LMA)` = `$85AD`), so it cannot drift from the bank it points at.
- `SND_ENGINE_TABLE_BANK` ($B), `SFX_BLOB_BANK` ($B), `SFX_ID_BASE` ($33),
  `SFX_TABLE_LEN` (135), `SFXID_REV_LOOP` ($AB) — **game config** (main.asm /
  config/sound_ids.asm / config/game.asm). The resident blob needs the game's
  bank/id layout to build; the AS side gets these from main.asm. Not engine
  sound contract, so correctly not in the authority.
- `FMVOLENV_COUNT` (3), `PSGVOLENV_COUNT` ($B) — **resident-blob-ONLY**. Grep of the
  whole tree finds NO definition; they exist only as `-D` values (referenced by
  `sound_psg.emp`'s `ld b, COUNT`). Homing them seam-local creates ZERO mirror.
- `FmVolEnvCtl_Loop/Sustain/Rest`, `PsgVolEnvCtl_Loop/Sustain/Rest` ($80/$81/$83) —
  vol-env control opcodes. Their canonical home is `engine/sound/sound_tables_z80.emp`
  (a GENERATED data module, "DO NOT EDIT BY HAND"). This is the **one genuine
  residual mirror** E2 leaves: dissolving it needs the generator (`gen_sound_tables.py`)
  to expose them `pub` or the authority to own them + the generator to import —
  out of E2's scope. Gap-ledgered.

## §4 — RETIREMENTS + RE-HOMES

Strict **2868 → 2868 / 0 / 4** (net 0). No test retired; no probe re-homed. The
change is internal to the harness emit path + one authority alias + one `.emp`
extern→use swap — the resident modules reference their consts as **bare names**
satisfied by the emit `-D` (only PROC imports use `use`), so sourcing the values
differently is transparent to every `.emp` module.

Retired hand-maintained surface:
- seam1.rs: the 5 `*_consts()` value functions (399 `(name, value)` entries) →
  5 `*_const_names()` name lists + `resolve_consts` (authority lookup).
- seam2.rs: the pinned `("DAC_SAMPLE_COUNT","10")`/`("DacSample_len","9")` equ
  carriers + their `assemble_equ_pairs` block → authority `-D`.
- dac_sample_tab.emp: `extern("DAC_SAMPLE_COUNT")*extern("DacSample_len")` (link
  assert) → `DAC_SAMPLE_COUNT * DacSample_len` (comptime fold via `use`).

## §5 — STEP-3 (retrospect) vs STEP-5 (engine opt)

- **Step-3:** the 399-value + 2-carrier hand-maintained surface is GONE; the only
  hand-maintained sound values left in the harness are the 14 `seam_emit_config`
  entries (8 of which mirror nothing — game config the emit must know, or
  resident-blob-only counts — and 1 is derived), plus the 6 vol-env control bytes
  (the one true residual, ledgered). The `dac_sample_tab.emp` guard moved from a
  link-time assert to a comptime fold, which is strictly stronger (it decides at
  lower, not link) and removed the carrier scaffolding.
- **Step-5:** none. Pure sourcing change; no lowering altered, no ROM byte moved.
  `sound_authority_consts` is memoized per aeon root (a `OnceLock<Mutex<HashMap>>`)
  so the ~30 per-emit resolves cost one module eval, not thirty.

## §6 — NEITHER-BUCKET HEADLINES

- **The authority needs `Z80_RAM` seeded.** `eval_all_pub_consts` does NOT follow
  `use engine.constants.{Z80_RAM}` from disk (it evaluates one file), so
  `SND_Z80_BASE = Z80_RAM` fails un-seeded. `sound_authority_consts` sources
  `Z80_RAM` from `engine/system/constants.emp` (its own sole authority, via the
  same eval) and seeds it — no hardcoded `$A00000`. Every value still traces to its
  authoring module.
- **`Z80_RAM` is the ONLY external the authority needs.** The Z80-space RAM
  addresses (`SND_SEQ_BASE=$1A00`, …) are absolute Z80 addresses, not
  `Z80_RAM + off`; only the 68k-side `SND_Z80_BASE` (absent from every seam1 table)
  depends on `Z80_RAM`. So the seam values never actually consume it — but the
  full-module eval must resolve every pub const, so it is seeded.

## §7 — GAP-LEDGER SWEEP

Two entries added (`campaign-gap-ledger.md`, "sound-constants E2 mirror
dissolution"):
1. The 6 vol-env control bytes — the residual seam↔generated-data mirror; a future
   harvest touches `gen_sound_tables.py`.
2. `FMVOLENV_COUNT`/`PSGVOLENV_COUNT` have no comptime source; ideally derived from
   the vol-env data length, which awaits the section-length primitive (row 1805).

## §8 — BOOKKEEPING

- **Spec** `2026-08-01-sound-constants-flip-design.md`: status → **✅ SHIPPED**
  (both parcels).
- **Census** `2026-07-31-conversion-tail-census.md` row #2 (row-59): → **✅ DONE**
  (E1 + E2 shipped, byte-identical, 2868/0/4).
- **Kill-list** `twin-scaffolding-kill-list.md`: row **97** added and CLOSED — the
  seam-1 399-entry mirror + seam-2 DAC carriers, with the residual noted.
- **Gap-ledger**: the two entries above.

## §9 — GATES (failures-first)

| gate | result |
|---|---|
| `emit_sound_blob` 15-blob artifact diff (before→after) | **0 drift** at every step |
| full strict `cargo test --workspace` (SIGIL_STRICT_GATE=1) | **2868 / 0 / 4** |
| `native_full_rom` (s4 plain+debug byte identity) | 3 / 0 |
| `native_offcanonical_full` (demo{,_debug} + config_a/b) | 7 / 0 |
| build.sh s4 `6cf74e65`/412127 · DEBUG=1 s4.debug `16615e46`/421958 | ✓ |
| seam colink + probes (`seam2_dac_head_colink`, `seam2_sfx_head_colink`, `seam2_seq_colink`, `seam2_soundtables_colink`, `seam2_pitchtable`, `seam1_native_link`, `tranche5`/`sfx`/`mt`/`sound_migration` negative probes, `seam2_colink_probe`) | all green |

## §10 — COMMITS (unmerged)

- aeon `sound-e2-mirror`: `sound_constants.emp` (+`FmPatch_fp_tl`) +
  `dac_sample_tab.emp` (extern→use).
- sigil `sound-e2-mirror`: `seam1.rs` (5 tables → name lists + authority harvest) +
  `seam2.rs` (DAC carriers → authority `-D`, `lower_emp_file` defines param) + this
  note + the census/spec/kill-list/gap-ledger bookkeeping.
