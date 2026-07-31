# Wave-B tile_cache #2 — the per-slot staging pointer

**Class PF / effort M.** Review §2 tile_cache #2. Second RAM-growing parcel (uses the
B-0b tail idiom).

## Size delta up front (the coordinator's flagged risk)

- **ROM SHRINKS**: plain −19 B (412376→412357), debug −51 B (422249→422198). The empty
  zero-loop and raw movem burst are deleted; the small pointer-store additions do not
  offset them. **`ASSEMBLED_LEN` unchanged in BOTH shapes** — the pre-bank $10000 margin
  is NOT consumed; if anything this parcel frees a little. `deform_pointer_*` and the mt
  gates PASS (the B-0 bank-anchor path is untouched — finding #1 clear).
- **RAM grows +$340 (832 B) at the TAIL**: `Block_Stage_Ptrs` (ds.l 16 = 64 B) +
  `Block_Stage_ZeroPage` (ds.b 768). Zero engine-RAM churn — only Engine_RAM_End and
  game RAM shift +$340 (8 game-side player pins repinned; no engine pin moved).
- **The 768-byte zero page is RAM, not ROM (deviation from the review — see below).** A
  ROM zero page would need 768 contiguous bytes that do NOT fit the ~0x1F0 (496 B) debug
  pre-bank margin, forcing a new anchored data island + refreeze bootstrap. The RAM tail
  is appendable with zero churn and the same read-only contract.

## The mechanism

The staging system decompresses/copies blocks into 16 RAM slots
(`Block_Stage_Buffers`, 768 B each) and consumers read the block through the slot base
returned in `a1`. Three claim classes each paid a full-block write:
`.empty_block` zero-filled 768 B (`clr.l`×192 ≈ 5.8k c), `.raw_direct` copied the
uncompressed ROM block into the slot (24× `movem.l` ≈ 4.0k c), compressed decoded via
S4LZ.

New: a **per-slot data pointer in RAM** (`Block_Stage_Ptrs[16]`), written at claim time,
returned by `FindStagedBlock` instead of the static ROM `BlockStage_PtrTable`:

- **empty** → point the slot at the shared `Block_Stage_ZeroPage` (768 read-only zero
  bytes); no zero-fill.
- **raw** → point the slot straight at the uncompressed ROM block (slot layout: NT 512 +
  collision 256, word-even — the existing DEBUG assert guarantees it); no copy.
- **compressed** → decode into the RAM slot as today; the pointer = the slot base (stored
  as the default at claim, before the branch, so it survives the S4LZ clobber).

`DecompressBlock` preserves slot×4 in d6 across the claim so the empty/raw exits can
index `Block_Stage_Ptrs`; the default (slot-base) store covers the compressed path.

### Behaviour identity (the correctness proof)

The staged pointer is **strictly read-only** in every consumer — verified:
`CopyBlockColumn` reads it via a0 (`movea.l a1,a0`) and writes only to
Tile_Cache_Nametable/Collision (a2); `FillRow` derives its read sources (a0/a3) from a1,
then re-points a1 at the cache dest (a4) before any write. So pointing the slot at ROM /
the zero page is safe. The block DATA delivered to consumers is **byte-identical**:
empty → zeros (identical to a zeroed slot); raw → the ROM block (identical to the bytes
the movem would have copied); compressed → the slot (unchanged). Therefore
Tile_Cache_Nametable / Tile_Cache_Collision and everything downstream (plane buffer →
VRAM) are byte-for-byte identical OLD vs NEW.

Verifier coverage (review's list): nothing writes through the staged pointer (proven);
`FindStagedBlock` switched ROM-table→RAM-array; slot reuse overwrites the pointer
unconditionally (every claim path stores it); the raw ROM block is word-even and
BLOCK_RAW_SIZE in slot layout; empty keys never false-hit ($FFFFFFFF is unreachable as a
real key — block_index ≤ 255).

### The zero-page contract (the RAM deviation's obligation)

`Block_Stage_ZeroPage` lives in the boot-cleared 64KB Work RAM (`boot.emp` clears all of
it, wrapping to $FFFF0000) and is **never written** — only the empty-block pointer
references it, and only to store the pointer, never to write the page. So it stays zero
for the ROM's life. A DEBUG zero-check assert was prototyped but **dropped**: it pushed
the debug tile_cache region 0xE past the B-0 0x40/section spread (branch-relaxation
amplified it). The contract is documented in `ram.asm` and enforced structurally (no
writer exists) rather than by a runtime check.

## Build + gate results

- **Both shapes build**: plain `crc=6a69403c len=412357`, debug `crc=7ea5e77d
  len=422198`.
- **Owned gates GREEN**: `tile_cache_port` (region + two-module flip; the new RAM
  symbols `Block_Stage_Ptrs`/`Block_Stage_ZeroPage` added to repin.toml + both test
  compositions), `section_port`, `collision_lookup_port`, `pins_rs_is_current`,
  `generated_pins_match_the_hand_typed_baseline`, `deform_pointer_*`, `test_p1` (the
  split player pins — see below), `ram_packing_invariants` (RAM stayed even + contiguous).
- **12 gates RED — all golden-ROM / frozen-table, REFREEZE-PENDING (yours)**:
  `native_full_sonic4_{plain,debug}` (whole-ROM size/bytes: 412357 vs golden 412376 — the
  message literally says "re-freeze the golden?"), the off-canonical
  `{config_a,config_b,demo_plain,demo_debug,flipped_config_a}` `anchor_matches_golden` +
  `full_file` + the `config_b` t24 (shared-engine tile_cache/ram change shifts every
  target). No engine region gate broke (RAM tail + alignment-absorbed ROM shrink).

### Broken-coincidence sweep (finding #2)

The +$340 game-RAM shift split three page-aligned player ring buffers that had been
plain==debug (`Player_Ring_Index`/`Pos_Ring`/`Stat_Ring`: were 0xB400/0xB200/0xB300 both
shapes → now plain 0xB700/0xB500/0xB600, debug 0xB800/0xB600/0xB700). **Benign**: their
only consumer, `test_p1_player_port`, already reads `.plain` and `.debug` separately per
shape (no shared-base assumption), and it passes. Swept the other game-RAM pins — no
shared-literal consumers.

## A/B PLAN (for the overseer's profiler run)

**State-identity zones (this is a PS-grade byte-identity parcel by construction):**
- `Tile_Cache_Nametable` (RAM, 9600 B) + `Tile_Cache_Collision` (RAM, 4800 B) —
  byte-identical OLD vs NEW at ≥3 anchor frames spanning a block-staging event
  (a block-column crossing, every 8 frames at 16 px/f). This proves the pointer
  indirection delivers identical block data.
- The visible plane (screenshot `cmp`) on a scroll-crossing frame — the VRAM tile cache
  nametable is downstream of Tile_Cache_Nametable via the plane buffer.
- Use the NEW debug ROM (`aeon/s4.debug.bin`, crc 7ea5e77d, branch `opt-wave-b3`).

**Anchors (code-point, self + inclusive):** `TileCache_DecompressBlock`,
`TileCache_FindStagedBlock`, `S4LZ_DecompressDict` (to isolate the non-decode delta),
`Tile_Cache_Fill`; `Lag_Frame_Count`.

**Expected deltas:**
- `DecompressBlock` self-time (inclusive minus `S4LZ_DecompressDict`) drops by the
  eliminated empty zero-fill (~5.8k/empty block) + raw copy (~4.0k/raw block). S4LZ
  itself is unchanged.
- Tile cache RAM/VRAM byte-identical (the correctness leg).

**HONEST win bound — the max-H drive may NOT show the bar.** On the committed
`profile_BEFORE_maxH.json`, `DecompressBlock` = 5226 c/f inclusive but **4259 of that is
`S4LZ_DecompressDict`** (compressed decode, which this parcel does not touch). The
non-S4LZ remainder is only **~967 c/f**, and part of that is per-claim overhead I keep.
So the empty+raw savings on max-H (camera 2016→4416, mid-level) are **bounded by ~967
c/f and likely well under** — max-H stages mostly compressed blocks, few empty/raw.

**The win case needs an empty/raw-heavy drive (say so loudly):** the review states empty
blocks "recur at world edges and blank regions — exactly where max-speed scroll runs."
The bar-clearing measurement wants a scroll INTO a world edge / blank band (many empty
blocks → up to `BLOCK_DECOMP_BUDGET`(6) × 5.8k = ~34.8k c/f worst case reclaimed), or a
raw-block-heavy region. **Recommendation:** run the A/B on (a) max-H for the identity
proof + the modest steady-state number, AND (b) a world-edge / blank-region scroll to
size the real win. If neither clears ~1k, the three-leg sub-bar KEEP test applies —
this parcel is **byte-identical-by-construction with ZERO adverse** (a pure removal of
work; the only added cost is ~3 pointer stores per claim, a handful of cycles), so leg 2
is maximally satisfied; legs 1/3 rest on finding the empty/raw-heavy drive. If even that
is sub-bar, it is a clean structural improvement (also retires the §2.5 `clr.l (a0)`
conventions violation) — your adjudication with the numbers.

## Ledger candidates / honest bounds

- **RAM zero page vs ROM (deviation from the review).** Chosen for buildability: a ROM
  zero page (768 B) exceeds the debug pre-bank margin (~496 B) and would need a new
  anchored data island + refreeze bootstrap. The RAM page is tail-appendable (zero
  churn) with the same read-only contract, but relies on "boot clears it, nothing writes
  it" rather than hardware immutability. If you prefer the ROM form for robustness, it is
  a refreeze-adjacent change (a new anchored 768-B zero section past $10000). Logged.
- The empty/raw win is drive-shape-dependent (above); the census effort model (M) priced
  the RAM-layout ripple, not the drive's block-class mix.
