# A/B evidence — art-streaming P2b Task 5: format cutover (64-tile pages, manifest v2, logical indices)

**Parcel:** `art-streaming-p2-task5` (sigil chain entry 59). The P2b format
cutover: 64-tile ZX0/raw pool pages, manifest v2, per-section local→global
translation. This is the section-GROWING mirror of chain-46 `objtest-gate` (a
section-REMOVING parcel) — the packer's second documented >ANCHOR_GAP HAND RULING.

## What changed

Aeon (`feat/art-streaming-p2`, commit `d3e51a1` + the generator `e28a471`):
- **Generator** (`tools/ojz_strip_gen.py`): `ART_POOL_PAGE_TILES` 256→64; Pass 5
  now writes per-section LOCAL tile indices (bits 0-10; pal/pri/flip untouched) +
  emits per-section `sec{N}_local_map.bin` (u16 BE `map[local]=global`) and the
  generated `sec_local_maps.emp` (`OJZ_Sec_LocalMaps: [*u8;9]`, indexed by flat
  section id); Pass 7 emits a deterministic JSON sidecar `{tiles, pinned}` (pinned
  = ≥75%-of-sections + page 0 always). Pool-tile ceiling → page-count ceiling.
- **regenerate-level.sh**: per-page ZX0/raw election (keep .zx0 iff ≥10% saving)
  + emits the manifest v2 table `OJZ_Act_Pool_PageTable: [PageManifest;N]`
  ({source, tiles, form, flags}) replacing the old longword ptr table.
  **verify_level_bin.py**: v2-aware (page size 2048, per-page form/wrapper).
- **Engine**: `constants.emp` (ART_POOL_PAGE_TILES=64, ART_POOL_PAGE_BYTES(_SHIFT),
  manifest form/flag consts, PAGE_TABLE_MAX); `structs.emp` (new `PageManifest`
  8-byte struct; `Act.act_sec_local_maps: *u8` at $22 — avoids growing the
  66-byte stride-locked `Sec`); `ram.emp` (the init-only
  `Art_Staging_Buffer = alias(Tile_Cache_Nametable)` DELETED → a dedicated
  `[u8;2048]` in lower_ram, +2048 B lower-RAM); `page_in.emp` (manifest-v2 stride-8
  read, manifest-driven form dispatch + DEBUG wrapper-agreement assert, dest from
  ART_POOL_PAGE_BYTES_SHIFT, size-0 guard — the three T4-review tripwires);
  `load_art.emp` (incremental enqueue-and-drain, no `@discards`); `tile_cache.emp`
  (translation at block decode: `TileCache_DecompressBlock` stashes the section's
  local-map ptr in d7 — which `S4LZ_DecompressDict` preserves — and `.translate_slot`
  patches the staged block's nametable words local→global before either copy loop
  reads them; empty/zero-page skipped, raw-direct copies-then-translates).
- **map.toml**: `OJZ_Sec0_LocalMap` added to `order` (between OJZ_Sec0_Blocks and
  OJZ_Palette). **act_descriptor.emp**: `act_sec_local_maps: OJZ_Sec_LocalMaps`.
- Full donor re-bake ran; the byte-repro invariant holds: all 589,824 nametable
  words, translated new-LOCAL→global through each section's `sec{N}_local_map.bin`,
  reproduce the OLD GLOBAL strips exactly (0 mismatches). Every non-art-pool output
  (collision, entity, palette, BG, source strips) is BYTE-IDENTICAL to master.

Sigil (`master`), this parcel:
- **Registry** (`native.rs`): new section `m!("games.sonic4.ojz_sec_local_maps_act1",
  "sec_local_maps", pins::SEC_LOCAL_MAPS)` between sec_block_blobs and ojz_act_assets.
- **repin.toml**: split `sec_block_blobs` (`OJZ_Sec0_Blocks`..`OJZ_Sec0_LocalMap`)
  + new `sec_local_maps` region (`OJZ_Sec0_LocalMap`..`OJZ_Palette`) + the
  `OJZ_Sec_LocalMaps` cross-seam symbol (act_descriptor_port).
- **measure_or_spread spread widened 0x100→0x400** (native.rs) — the sanctioned
  MEASURING-device widen (its own comment: "never moves an unchanged section"),
  needed because the pool grows 0x33A in ONE section, past the old 0x100 adjacent
  step. ANCHOR_GAP stays 0x400 (a placement/classification guard — NOT touched).
- **ojz_run_b_port.rs**: `sec_local_maps` added to the region byte-gate.
- pins.rs (regenerated), repin_pins hand baseline (lockstep), the seven goldens +
  size tables + provenance (refreeze --ab).

## The HAND RULING (mirror of chain-46, section-GROWING direction)

The 64-tile cutover shifts every section downstream of the pool, up to the DAC
org-anchor (0x48000), by a delta that **exceeds ANCHOR_GAP (0x400)** — the first
such shift in the art-streaming chain (T2 +0x80, T3 +0x2F0, T4 +0x180 all stayed
under it). Measured downstream deltas (from the converged resolve):
- **pool region +0x33A** (3 ZX0 pages → 10 + the 8-byte×10 manifest-v2 stride):
  `OJZ_Act1_Descriptor`, `OJZ_Sec0_Blocks` shift +0x33A.
- **+0x1026** (pool +0x33A + the new `sec_local_maps` island 0xCDC, plus align):
  `OJZ_Palette`, `BgAnim_Table`, `Map_TestObj`, `Ani_Sonic`, `HeightMaps`,
  `Ani_Particle`(debug) shift +0x1026.
- new head `OJZ_Sec0_LocalMap` inserted; `SEC_LOCAL_MAPS` len 0xCE0/0xCDC.
- **Org-anchored islands UNTOUCHED**: `Dac_Temp_Blip` 0x48000, `SoundTablesZ80_Head`
  0x58000, `Song_MovingTrucks` 0x58607, and all post-sound sections
  (GameState_*, Replay_OJZ_Fixture, ReleaseFault/BusError, EndOfRom). The growth is
  absorbed by the pre-DAC gap (OJZ data ends well below 0x48000). The error_handler
  island remains the LAST emission (MDDBG locator invariant).

**Recipe (which tables were reseeded, exactly what chain-46 did in the shrink
direction):** the canonical repin/derive walk seeds provisional bases from
`golden/offcanonical_sizes/s4.txt` + `s4_debug.txt` (`SizeSource::Frozen`), and the
org-hole guard `packed > p + ANCHOR_GAP` reads them. Those two tables were
hand-shifted for the downstream run by the measured deltas (transient — enough to
bring `p` within ANCHOR_GAP of the packed truth so the walk reaches its fixpoint),
then `refreeze --freeze` re-derived every value EXACTLY. Alignment class preserved
(t24(b): the OJZ data sections are even-aligned; the deltas are even). No
ANCHOR_GAP change ships.

## A → B

- Baseline aeon (A): master, 3× 256-tile ZX0 pages, global nametable indices.
- B (this parcel): 10× 64-tile pages (all elected ZX0; raw arm dormant), per-section
  local indices + translation. s4.debug.bin crc=c70bc157 len=425538 (fresh build).
- The controller's oracle pass (pixel-identical boot + max-scroll circuit +
  collision + init-stream counters) is the runtime proof; the build-side proof is:
  all shapes green, byte-repro of the translation (0 mismatches), the six golden
  gates + ojz_run_b_port (incl. sec_local_maps) + act_descriptor_port + repin_pins
  + refreeze --check.

## Follow-up: self-test manifest-v2 walk (crash fix, 2026-08-08)

Oracle boot ADDRESS-ERRORed at `CompressionSelfTest.eq_page` (faulting 0x400001,
odd/past EndOfRom): the DEBUG-boot ZX0-vs-ZX0R equivalence walk was the THIRD
manifest consumer (after page_in + act_descriptor) and still strided
`act_art_pool_table` as the OLD longword pointer array (stride 4), so it walked the
stride-8 `PageManifest` records as garbage pointers. Fixed in
`engine/debug/compression_selftest.emp`: the `.eq_page` walk now strides
`sizeof(PageManifest)=8`, takes the source from `pm_source` and length from
`pm_tiles*32`, SKIPS `pm_tiles==0`, and equivalence-tests ONLY
`pm_form==ART_PAGE_FORM_ZX0` pages (a raw page is skipped — ZX0R would misparse raw
bytes). This also RESOLVES DEFERRED_WORK item (b) from the Task-3 review
("self-test doesn't assert form") — now fixed, not deferred.

DEBUG-shape-only change (the walk is inside `if HAS_ACT_ART_POOL == 1`, DEBUG-only
module): s4.debug.bin crc c70bc157→f6a5fe12 (len UNCHANGED 425538 — no layout
shift, no pin/table change), config_a.bin re-frozen. plain s4.bin (dcb6c78f), demo,
demo.debug, config_b, lean are BYTE-UNCHANGED (verified md5-identical). Debug
goldens refrozen; chain 60.
