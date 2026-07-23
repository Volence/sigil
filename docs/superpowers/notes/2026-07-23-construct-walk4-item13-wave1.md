# Construct-walk #4 — item-13 prelude domain-type pass, wave 1 (RATIFIED)

> **STATUS: DESIGN RATIFIED 2026-07-23 (Volence, at the gate) — implementation NOT scheduled
> yet** (its own small byte-neutral parcel, G5 template; runs after the §D backlog arc closes,
> NOT folded into it and NOT into t18's overnight run). Seeded from the item-13 first-wave
> ledger rows (941c5f4); G5's enforcement surface ([call.slot-type-mismatch], as-bless,
> out(dN: Type)) is the substrate.

## Ratified wave-1 types (all `pub newtype`, engine/system/types.emp)

1. **`SongId = u8`** + **`SfxId = u8`** — DISTINCT. Naming ruled: **SongId** (matches the
   `SONG_*` constant family; the API name `Sound_PlayMusic` describes the ACTION, types
   describe the VALUE — both stand unchanged). SFX side: `SfxId` matching `SFX_*`. Ids are
   const-born: typing the `SONG_*`/`SFX_*` constant definitions makes every table and call
   site typed with near-zero ceremony. Closes the wrong-sound class; reinforces the
   H-1-hardened sound seam. 68k-side only (the Z80 driver is untouched — the types stop at
   the param-block boundary).
2. **`AnimId = u8` + `AnimFrame = u8` + `MappingFrame = u8`** — THREE types, ruled over the
   cheaper two-type cut. The script-frame vs mapping-frame pair is the highest-risk swap
   (both small indexes, SST-adjacent fields); G5's checker exists for exactly this. Known
   cost, accepted at ratification: ~1-2 `as`-re-blesses at the animate advance site (frame
   increments degrade under strict-degrade — the moved-vs-computed line, paid knowingly).
3. **`VramTile = u16` + `VramAddr = u16`** — COMPTIME LAYER FIRST (ruled): `VRAM_*` layout
   constants typed VramTile; `vram_bytes()` takes VramTile → returns VramAddr; `vram_art()`
   takes VramTile. Every PLC entry / art_tile spelling checks for free — zero register
   ceremony. Register-slot typing (DMA-queue params etc.) is a LATER wave, judged after the
   comptime layer's feel is known.

## Standing boundaries (re-affirmed at this walk)

- Wave 2 (Tile/Block/Chunk indexes, Coord/Velocity out-typing, dimensional S2-D8) stays
  A4-i-GATED — hot-loop shift/add chains make typing them ceremony noise today (ledger parent
  row carries the rule: MOVED+COMPARED pays, shift/add-chain waits).
- The FlatIDXY.d2 verifier-gap row is unchanged; nothing in wave 1 carries a typed value
  across a conditional-save callee.
- Implementation parcel bar (when scheduled): byte-neutral HARD (dual-invocation canonical
  EXACT), strict green + new unit/corpus tests per type family (incl. a negative
  SfxId-into-SongId-slot pin and an AnimFrame/MappingFrame swap pin), port-loop step-2
  type-layer walk item cites this note.
