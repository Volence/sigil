# Tranche 16 — `tile_cache.asm` port: step-0 recon + design note

**Status:** step-0, owed at the Fable gate BEFORE any code.
**Target:** `aeon/engine/level/tile_cache.asm` (1290 lines, 14 procs) — the 2D
block-decompression tile cache (§4.7), section.emp's paired streaming sibling.
**Precedent-setting:** first port whose STEP-5 is the tranche's headline (the
file is the measured lag driver), not a byte-faithful transcription with an
optional optimize pass.

---

## 1. Recon

### Procs (14)
| Proc | Role | Hot? |
|---|---|---|
| `Tile_Cache_GetTile` | nametable-word lookup for a world tile (×80 shift-add) | called by render |
| `Tile_Cache_GetCollision` | collision-byte lookup + layer plane select | **flip seam** (collision_lookup.emp tail-calls it) |
| `BlockStage_PtrTable` | ROM ptr table via `rept BLOCK_STAGE_SLOTS` | data (step-4 construct cand.) |
| `TileCache_FindStagedBlock` | probe staging slots (`dbeq`) | per-block |
| `TileCache_InvalidateStaging` | empty slots | init |
| `TileCache_DecompressBlock` | decompress 1 block → slot; **`if Sec_len<>66`** stride assert; raw-copy movem burst; zero-fill | per-block (cold-ish) |
| `TileCache_CopyBlockColumn` | vertical run copy; **`collSrcRowBase`** macro; circular wrap | per-column |
| `Tile_Cache_Init` | populate initial viewport | init |
| `TileCache_FillAll` | full fill | init |
| `Tile_Cache_Fill` | **per-frame** entry: frame gate, budget, resume ladder | **HOT** |
| `TileCache_FillColumn` | one column, budget-limited | hot (columns) |
| `TileCache_FillRow` | one row (NT + collision) | **HOTTEST — the lag lever** |
| `TileCache_HSlide`/`VSlide`/`VSlideUp` | O(1) circular evicts | hot (cheap) |
| `TileCache_Reinit` | recovery re-center+refill | **no callers — feature-dead** |

### Region (shape-VARYING — DEBUG asserts in the raw-copy path)
- **plain:** `[Tile_Cache_GetTile $42FA … Collision_GetType)` — `TileCache_Reinit`
  @ $4BBC; region ≈ **$900**. Exact end pinned at the step-1 build.
- **debug:** `[$4EC4 … )` — `TileCache_Reinit` @ $583E. Debug carries ≈$B8 extra
  (the `assert.l`/`assert.w` block at donor :221-224). **Not** shape-invariant
  (contrast load_object) — byte gate runs BOTH shapes independently.
- **Placement:** `engine.inc:312`, currently UNGATED, between `plane_buffer`
  (:311) and the already-gated `collision_lookup` (:313, resume org plain
  $4C42 / debug $58C4). Immediately upstream of collision_lookup.emp → the
  re-pin wave from steps 2-5 propagates down through collision_lookup's resume
  orgs, section, camera, and everything below.

### Dependencies
- **Externs (AS-side, link fixups):** `S4LZ_DecompressDict`
  (`engine/compression/s4lz_decompress.asm` — NOT ported; extern), the RAM
  symbols (`Cache_*`, `Block_Stage_*`, `Tile_Cache_Nametable/Collision`,
  `Camera_X/Y`, `Current_Act_Ptr`, `Frame_Counter`, `Section_Plane_Dirty`).
- **Sec/Act struct fields:** `Sec_len`, `Sec_sec_block_index`,
  `Sec_sec_block_dict`, `Sec_sec_block_dict_len`; `Act_sec_grid_ptr`,
  `Act_grid_w`, `Act_grid_h` → **3rd .emp consumer of Sec/Act** (see DQ1).
- **Consts (engine.constants twin):** `TILE_CACHE_{COLS,ROWS,STRIDE,NT_SIZE,
  COLL_ROWS,COLL_SIZE,COLL_PLANES,MARGIN_H,MARGIN_V}`, `BLOCK_{STAGE_SLOTS,
  RAW_SIZE,NT_SIZE,TILE_SIZE,TILE_SHIFT,COLL_COLS,COLL_PLANE_SIZE,INDEX_SIZE,
  DECOMP_BUDGET}`, `VFILL_ROWS_PER_FRAME`.
- **Macros:** `collSrcRowBase` (DQ2), `assert.l/.w` (shipped), `ifdebug` →
  `if DEBUG == 1 {…}` (shipped).

---

## 2. Hazard sweep (step-0 trip-check) — ledger grep for the file + its symbols

| Row | Condition | This port |
|---|---|---|
| **1057** | tile_cache = vertical-streaming lag driver (`TileCache_FillRow` 48.9k cyc/f, 38.2%) | **FIRES — this is the step-5 headline** (DQ5) |
| **1051** | Sec/Act shared-struct module — trigger met at t15, deferred on size | **FIRES — 3rd consumer** (DQ1) |
| **1052** | VDP-macro shared home — "2nd consumer ports → shared home" | **DOES NOT FIRE — correction owed** (see below) |
| **1053** | VDP-register-const candidate | N/A (no VDP writes here) |
| **161** | `Tile_Cache_Nametable` hand-size `if` + INIT-ONLY lifetime | ram.asm-side (buffer decl); tile_cache CODE doesn't declare it — out of this port's scope, ram.asm's own port |
| **477** | `Tile_Cache_GetCollision` tail-call + attribute precedent | informs DQ3 (the flip is a tail-call from collision_lookup.emp) |
| **861** | synthetic LABEL VMAs in port tests (`Tile_Cache_GetCollision`) | step-1 harness: the flip test + ojz_scroll driver need synthetic VMAs |

**LEDGER CORRECTION (row 1052):** the row asserts "plane_buffer/tile_cache/
load_art all use vdpComm/vdpCommReg AS-side." **False for tile_cache** — it
writes only RAM buffers (`Tile_Cache_Nametable`/`Tile_Cache_Collision`), zero
VDP command words. The `engine.vdp` shared-home 2nd-consumer trigger is NOT
fired by this tranche; it still waits on plane_buffer or load_art. Amend row
1052 to drop tile_cache from its consumer list at the gate.

---

## 3. Design decisions

### DQ1 — Sec/Act: file-local mirror, DEFER the shared struct module
tile_cache is the **3rd** reg-relative consumer of Sec/Act (after
entity_window + section). Row 1051 already parks the shared `engine.structs`
module as its own sst-usability-style batch, "deferred on tranche-size, NOT
re-gated on a fired condition."

**Decision: follow section's precedent exactly** — mirror the 7 Sec/Act fields
as drift-locked offset consts + the `if Sec_len<>66` stride guard as an
`ensure(Sec_len == 66, …)`, file-local in tile_cache.emp. Do **not** ship the
shared struct module inside t16.

*Rationale:* this is already the biggest region ported (~2.4× section);
bundling the shared-struct unwind (which collapses section + entity_window +
act_descriptor + tile_cache offset consts in one wave) violates keep-tranches-
small and tangles four files' re-pins into one branch. The file-local mirror
is byte-neutral and drift-locked (the guard is the lock). **Action:** strengthen
row 1051 to "3rd consumer confirmed" — the shared module is now the clear next
sst-usability batch after t16 merges.

*Flagged for Volence:* this is the one genuine SCOPE call. If you'd rather t16
force the shared-struct module (it IS the natural forcing point at 3 consumers),
say so and I'll re-scope — but my recommendation is the small-tranche path.

### DQ2 — `collSrcRowBase` → file-local comptime-fn (macro-port rule)
Donor macro body: `lsr.w #1,reg` (tile row → collision row) then `lsl.w #4,reg`
(×`BLOCK_COLL_COLS`), guarded by `if BLOCK_COLL_COLS<>16 error`. Two sites
(donor :314, :1089), both parity-safe by construction.

**Macro-port rule analysis:**
- *Wrong-input scan:* the donor takes a bare register; the `.emp` counterpart
  takes a **typed `Reg` param** (frames.emp/`frame_piece_count`,
  `perform_dplc` pattern) — the only thing a caller can get wrong is the reg,
  and the type names it.
- *Guard upgrade:* `if BLOCK_COLL_COLS<>16 error` → comptime
  `ensure(BLOCK_COLL_COLS == 16, "…")` inside the fn (drift-locked to the const
  twin). The runtime-impossible input is gone by construction.
- *Home:* **file-local** in tile_cache.emp — single-file consumer (contrast
  frames.emp, which is a shared module because frame_piece_count spans
  load_object + animate). Ratified complement to the EntityScanState file-local
  precedent (single consumer → lives with consumer).
- *AS twin:* keeps the inline `collSrcRowBase` macro spelling (lockstep is
  byte-level, sprites/animate precedent).

**Ships at step 1** (demanded — the file can't port without a counterpart).
Signature: `pub comptime fn coll_src_row_base(reg: Reg) -> Code`.

### DQ3 — ownership FLIP: `collision_lookup.emp` ↔ `tile_cache.emp`
`collision_lookup.emp` (already ported/gated) tail-calls `Tile_Cache_GetCollision`
(its twin: `bra.w Tile_Cache_GetCollision`). Porting tile_cache flips that
symbol's ownership AS→.emp. Per the port-loop **proof-mechanism feed-forward**
(the symbol-ownership FLIP class, section/entity_window is the template): owe a
**persisted two-module link test** proving `collision_lookup.emp` resolves
`Tile_Cache_GetCollision` against `tile_cache.emp` when both gates are on.

Mixed-build matrix (both gates independent): the seam symbol must resolve in all
four combos — {tile_cache AS|emp} × {collision_lookup AS|emp}. `a1`/`d0-d2`/`a0`
contract across the tail-call is the seam; `d3.b`=layer passes through.

### DQ4 — `BlockStage_PtrTable` `rept` → step-4 construct candidate
`rept BLOCK_STAGE_SLOTS { dc.l Block_Stage_Buffers + i*BLOCK_RAW_SIZE }` — an
arithmetic-progression ROM ptr table. NOT `offsets` (that's `Target-Base`
self-relative); this is `base + i*stride` absolute longs. Step-4: adopt (does a
comptime-loop emit / a small table helper fit?) or build a `ptr_table` helper,
or keep the `rept` transliteration if no construct reads better. **Decide at
step 4, not now** — flagged as a construct-pass candidate.

### DQ5 — step-5 lag lever (THE deliverable)
Row 1057: `TileCache_FillRow` 48.9k cyc/f (38.2%), row path ~2× column path.
**Concrete lever already visible in the donor** — `TileCache_FillRow`'s inner
`.fr_col_loop` (donor :1097-1152) reloads **three loop-invariant absolute-base
`lea`s PER CELL**:
- `:1130 lea (Tile_Cache_Nametable).l, a1`
- `:1141 lea (Tile_Cache_Collision).l, a2`
- `:1147 lea (Tile_Cache_Collision+TILE_CACHE_COLL_SIZE).l, a2`

These bases never change across the row — classic invariant-ladder hoist
(iteration→row scope). Candidate: hoist to row-scoped a-regs (register-pressure
permitting — the loop already juggles a0-a3; a real analysis at step 5).
Secondary: the per-cell circular column wrap (`add origin / cmpi / sub`) may be
strength-reducible. **Behavior-neutral cycle win → LIVE verification required**
(oracle, OJZScroll vertical 8px/f — the exact t15 profiling scene; lag-frame
counter is ground truth, not the profiler alone). Detailed per-proc step-5
interrogation table at step 5; headline recorded here.

### DQ6 — DEBUG diagnostics (standard, shipped constructs)
- `if Sec_len<>66 error "…"` → comptime `ensure` (DQ1, drift lock).
- `ifdebug assert.l d0, hs, #BLOCK_INDEX_SIZE` (+ the word-even assert) →
  `if DEBUG == 1 { assert.l … }`, operand spellings identical to the twin
  (auto-message embeds them). Self-gates to zero plain bytes — the source of
  the debug/plain region-length divergence.

### `TileCache_Reinit` — feature-dead, FLAG-not-cut
No callers; donor comment: "retained as the documented recovery mechanism for
future cache-miss / debug-warp handling." Step-4 verb (d): deliberate/forward-
scaffolding dead code → **flag to Volence, never auto-cut** (the
`AnimateSprite_PerFrame` precedent). Port it faithfully; note it in the packet.

---

## 4. Step-1 plan (transcribe)
- New `engine/level/tile_cache.emp`; 1-1 faithful (same mnemonics, explicit
  widths, comments carried).
- `SIGIL_EMP_TILE_CACHE` gate around `engine.inc:312`, mirroring section's
  block (resume org = collision_lookup's Collision_GetType start, per-shape).
- **Demanded feature:** `coll_src_row_base` comptime-fn (DQ2).
- Region pin (both shapes) + byte gates BOTH shapes (region is shape-varying).
- **Flip proof:** two-module link test collision_lookup.emp↔tile_cache.emp
  (DQ3) — the gate artifact named in the packet.
- Mixed-build acceptance (4-combo gate matrix) + negative probes + gate-off
  neutrality (both shapes rebuild to canonical 11382fa7/36bf0f17).
- Harness: `tile_cache_port.rs` both shapes; synthetic VMAs per row 861.

## 5. Probes (binding-class stated — probe-fidelity rule)
- **P1 (flip, LINK-TIME):** with both gates ON, `Tile_Cache_GetCollision`
  resolves collision_lookup.emp's tail-call to tile_cache.emp's symbol — a
  link-time cross-module edge, NOT a comptime base. Test asserts the resolved
  address, both shapes.
- **P2 (region byte gate, both shapes):** plain + debug independently (shape-
  varying); debug exercises the assert block.
- **P3 (step-5, LIVE):** oracle OJZScroll vertical 8px/f, VInt_Lag + FillRow
  cyc/f before/after the hoist. Ground truth = lag frames, per genesis-dev.

## 6. Scope
One file (`tile_cache.emp`) + its gate + `coll_src_row_base` + the flip test +
the file-local Sec/Act mirror. Shared struct module, BlockStage table construct,
and any deeper step-5 restructure beyond the invariant hoist are OUT (deferred /
step-4-decided / measured-first). Keep-tranches-small holds even though this is
one big file.
