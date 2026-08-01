# Conversion-tail Parcel E — the sound-constants flip: INSPECTION STOP (reclass L → CAPSTONE)

Porter: conv-e-sound-constants (aeon + sigil branches created, NO code changes).
Outcome: **STOP at inspection** (the Parcel-C precedent). The census row-59 premise is
materially mis-scoped and one of its two stated goals is stale. Reported with numbers
below for the overseer/Volence to rule: build it as its own spec + parcels (the item-#7
RAM-regions path), or accept AS-authored `sound_constants.asm` as the standing "100%
.emp" exception. No clean sub-seam exists that is smaller than the whole flip.

## What the census said (row #2 / Parcel E)

> `engine/sound_constants.asm` — 1480 (321 defs) — "shared 68k/Z80 sound equates" —
> class **PORT (constants flip)** — effort **L** — "P5 mechanism; read by 5 sound `.emp`
> modules" — "BN … its own flip using the proven mechanism (harvest a `sound_constants.emp`
> + inject). The SND_* comptime-source circularity (**ledger 1619**) dissolves here." →
> files: **2**.

## Premise corrections (verified, with numbers)

### 1. The file is NOT a flat-equate holder — it is a mixed-construct Z80 module

`engine/sound_constants.asm` (1481 lines) contains, beyond flat `=` equates:

- **321 `=` equates** — but a large fraction are STRUCT-DERIVED aliases
  (`sc_stream_ptr = SeqChannel_sc_stream_ptr`, `sx_* = SfxChannel_sx_*`,
  `SFXH_* = SfxHeader_*`, `sc_env = SeqChannel_sc_psgenv`, …), not independent values.
- **5 `struct … endstruct` blocks**: `DacSample` (9 B), `FmPatch` (32 B),
  `SfxHeader` (8 B), `SfxChannel` (68 B), `SeqChannel` (60 B) — all **Z80-side**.
  `SfxChannel`/`SeqChannel` carry an intricate **13-field shared-prefix invariant**
  (the SFX struct's +0..+56 prefix must mirror `SeqChannel` field-for-field, asserted).
- **5 comptime `function` defs** (`ym_timerA_n_from_tempo`, `ym_timerA_period_ns`,
  `ym_timerA_hz`, `timerAReload`, `dac_rate_hz`).
- A **derived Z80 RAM layout** (~40 computed addresses: `SND_SEQ_BASE` → `SND_SEQ_END`
  = base + `CHROUTE_COUNT * SeqChannel_len`, `SND_FM_SCRATCH`, `Snd_SongBase`,
  `SND_MUSIC_PARAM`, `SND_SEQ_TRACE`, `SND_GLOBAL_EXPR`, page-aligned `SND_SFX_BASE`,
  `SND_SFX_CHAN_END` = base + `SFX_VOICE_COUNT * SfxChannel_len`, …). These mix flat
  const bases WITH `sizeof(struct)` — the two harvest families jointly.
- **~40+ real `error`/`fatal` assertion walls** (the MEV_ opcode collision matrix,
  the RAM-seam guards, the shared-prefix guard, Timer-A pins) that must re-home as
  `.emp` comptime `ensure`s.

This is the **structs-flip class (Parcel A) + a derived-RAM-layout class + a
flat-equate class fused in one file**, not the "constants/equate holder" the census
classed it as. Parcel A (`ece869a`) deliberately flipped only the ENGINE structs
(`Act/Sec/DMAEntry/Sst/…`) and left the 5 SOUND structs here untouched.

### 2. "Read by 5 sound `.emp` modules" understates a TRIPLE, UNGUARDED mirror

`sound_constants.asm` is not the single source it advertises. Its values are copied,
by hand, into **three parallel sinks with no cross-check between them**:

| Sink | What | Size |
|---|---|---|
| `engine/sound_constants.asm` (AS) | the nominal "single source of truth" | 1481 ln |
| `seam1.rs` — 5 hardcoded Rust tables | `driver_consts` 125 · `sequencer_consts` 112 · `sfx_consts` 75 · `fm_consts` 57 · `psg_consts` 30 = **399 `(name,value)` entries** fed as comptime `-D` to each resident Z80 `.emp` module's `emit_sound_blob` lower | 399 |
| `seam2.rs` — pinned equ carriers | the whole-ROM co-link's DAC-head stand-in (`DAC_SAMPLE_COUNT`/`DacSample_len`, comment: *"the co-link's stand-in for `sound_constants.asm`"*) | small |

Plus the **68k `.emp` link-extern consumers**: `sound_api.emp` (24 `extern(...)`),
`sound_debug.emp` (7), `dac_sample_tab.emp` (2) — resolved against the AS file at link.

So "retire the 5-consumer mirrors" = delete the **399-entry seam1 table set**, rewire
`emit_sound_blob`/`file_specs` to harvest, retire the seam2 pinned carriers, AND swap
the 33 68k-`.emp` externs for `use engine.sound_constants`. The AS-residual 68k `.asm`
side reads it **~0 times** (grep: 9 hits across 3 files, all comments or game-side
re-defs `SND_ENGINE_TABLE_BANK = MovingTrucks_Bank_Start >> 15`) — the sound stack is
fully `.emp`, so the true consumers are the `.emp` modules on both CPU sides.

### 3. The stated circularity (ledger 1619) is ALREADY SETTLED

Ledger **line 1877** (2026-07-30, seam-2 stage-3) explicitly marks **rows 1615/1619/1620
SETTLED**: row 1619 (generated-file ownership) was realized for `sound_tables_z80.asm`
via *"the generator emits `.emp`"* (`gen_sound_tables.py`), and row 1620 notes *"the SND_*
comptime-source half (blocker of the DAC head) was already dissolved at seam-2 stage-2b."*
The census cites 1619 as the thing Parcel E dissolves — but 1619 is a DIFFERENT file and
is closed. The still-live hazard the flip WOULD close is the **seam1 399-entry
resident-blob mirror** (ledger 1656 blocker-1's residue), which the census did not name.

## Why this is CAPSTONE-scale, not effort-L / files-2

The mechanism EXISTS (`harvest_engine_constants` + `harvest_engine_struct_offsets` +
`sizeof` in eval are all proven — this is NOT a missing-feature blocker like Parcel C's
region-`vars`). But the WORK is the item-#7 (RAM-regions) shape, not a mechanical port:

- A new `sound_constants.emp` (~250+ lines): 5 Z80 struct twins + the shared-prefix
  invariant + the derived-RAM layout (flat-const bases × `sizeof`) + ~40 `ensure` walls
  + 5 comptime fns — the FIRST Z80 struct-offset + derived-layout composition (Parcel A's
  struct harvester is proven for 68k structs; the derived-RAM-from-`sizeof` mix is new).
- Harvester extension: a `harvest_sound_constants` (flat) + sound entries in
  `STRUCT_OFFSET_TWINS` (offsets) + the derived-RAM eval, injected into BOTH the AS
  residual (guarded defines) AND — the new part — the resident Z80 blob build, replacing
  all 5 `seam1.rs` tables and the seam2 carriers.
- `emit_sound_blob`/`native_sound_blob`/`file_specs` rewired from `fn()->Vec` mirrors to
  harvested values; the 33 68k-`.emp` externs swapped to `use`.
- `engine.inc` include removed; `sound_constants.asm` deleted.
- Byte-identity bar across all 6 ROMs **+ the resident Z80 blob** (the blob's bytes come
  from the seam1 values — every harvested value must match the current mirror exactly, or
  the blob shifts).

There is **no clean sub-seam**: the derived-RAM addresses mix flat consts and struct
sizeof, and each seam1 table interleaves flat / struct-offset / derived-RAM / opcode
classes, so a partial flip (e.g. flat equates only) leaves the structs + derived RAM in
AS, does not delete the file, does not retire any seam1 table, and does not dissolve the
mirror. Same shape as Parcel C's "even the 9-line stub needs the whole feature."

## Recommendation

Path (1), the item-#7 precedent: build it as its own **spec + plan + 2–3 parcels**
(sound-struct twins & derived-RAM `.emp` → harvester extension & seam1/seam2 retirement →
68k extern swap & `.asm` deletion), each with the six-ROM + Z80-blob byte-identity bar.
Or path (2): accept AS-authored `sound_constants.asm` as the honest "100% `.emp`"
exception (like the vendored debugger, #6) — nothing is broken; the flip's payoff is
dissolving a hand-maintained 399-entry mirror, which is real but not urgent.

Either way the census row-59 needs reclass **L → CAPSTONE** and its ledger-1619 citation
corrected. No aeon or sigil code was changed by this porter; branches left clean.

## Gates / provenance

No build run (inspection-only STOP). Strict baseline unchanged at 2868/0/4. Chain-9 tips
untouched. This note + the census annotation are the only artifacts.
