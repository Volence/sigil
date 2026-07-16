# Tranche 17 — `plane_buffer.emp` port, STEP-0 DESIGN NOTE (for Fable's gate)

Region: `engine/level/plane_buffer.asm` (370 lines) → `plane_buffer.emp` — §4.1
the deferred nametable **plane buffer**: the VDP-side DRAW half of the streaming
engine (producers append column/row entries to `Plane_Buffer`; `VInt_DrawLevel`
drains them to the VDP inside VBlank). section.emp's paired *draw* sibling
(tile_cache = the cache-fill sibling; section = the orchestrator).

**Charter (the step-5 headline):** gap-ledger **row 1066** — the copy/draw-bound
H-crossing residual. `Draw_TileColumn` is plane_buffer's half of the horizontal
Wave-1-that-never-happened; the domain split names `Draw_TileColumn` (VInt-side
nametable draw) → plane_buffer, `TileCache_FillColumn`'s copy loop → tile_cache.

---

## 0. Baseline — VERIFIED before touching anything

| Item | Expected (prompt) | Observed | ✓ |
|---|---|---|---|
| sigil master | 7199792 | 7199792 (clean) | ✓ |
| aeon master | 6116254 | 6116254 (`?? …comfyui-art-pipeline-design.md` untracked spec only) | ✓ |
| plain ROM | 453087 / crc32 `b335bdc6` | 453087 / `b335bdc6` | ✓ |
| debug ROM | 461110 / crc32 `827e18c4` | 461110 / `827e18c4` | ✓ |

Provenance = **CRC32 + byte size** (campaign standard; the SHA1 slip cost the
unified-prefetch gate a round — [[provenance-is-crc32-plus-size]]). Strict suite
(`SIGIL_STRICT_GATE=1 AEON_DIR=<worktree> cargo test --workspace` = 194/2252/0) +
`repin --check` will be re-run in the **seeded worktree** at step 1 (paired-state
gate: `AEON_DIR` → the branch tree, never master).

---

## 1. File anatomy — 5 procs, one leaf region

| # | Proc | Lines | Role | Hot? |
|---|---|---|---|---|
| 1 | `Plane_Buffer_Reset` | 9–11 | zero `Plane_Buffer_Ptr` | trivial; **NO CALLERS** (flag §9) |
| 2 | `Draw_TileColumn` | 23–162 | append 1 tile COLUMN (plane A), row-63/0 wrap split | **YES — the charter proc** |
| 3 | `Draw_TileRow_FromCache` | 171–257 | append 1 tile ROW (plane A), ×80 shift-add source walk | **YES** (measured 5.6–11.7k) |
| 4 | `Draw_BG_TileColumn` | 270–313 | append plane-B column strip (§4.2), Sec/Act layout ptr | **NO CALLERS** (flag §9) |
| 5 | `VInt_DrawLevel` | 324–370 | drain buffer → VDP in VBlank; raw `$8Fxx` + cmd longs | VBlank hot |

**Region:** `Plane_Buffer_Reset .. Tile_Cache_GetTile` (contiguous — plane_buffer.asm
sits between load_object and tile_cache in `engine.inc`, currently included
UNCONDITIONALLY at `engine/engine.inc:311`).

**SHAPE-INVARIANT.** The file has **zero** `if DEBUG` / `ifdef __DEBUG__` / assert /
raise_error / ifdebug. Plain and debug bytes are identical length:
- plain: `0x405E .. 0x42FA` = **0x29C**  (base 0x405E = LOAD_OBJECT resume org, `engine.inc:308`)
- debug: `0x4C28 .. 0x4EC4` = **0x29C**  (base 0x4C28 = LOAD_OBJECT debug resume, `engine.inc:306`; end 0x4EC4 = `TILE_CACHE.debug_base`)

This is the **section.emp class** (shape-invariant, easiest byte gate) — NOT the
tile_cache/entity_window shape-varying class. Both shapes still gated; the debug
shape differs from plain only by the upstream base slide.

**Leaf status:** plane_buffer.asm calls NOTHING cross-seam (no `jbsr`/`jsr`/`bsr`;
`VInt_DrawLevel`'s only "calls" are VDP writes). It READS `Tile_Cache_Nametable`,
`Cache_*`, `Plane_Buffer[_Ptr]`, `Section_Right_Col_Written`, `Current_Act_Ptr`,
`Sec/Act` layout ptrs. ⇒ **no OUTBOUND ownership flips** — plane_buffer never
re-resolves a call into another .emp module.

---

## 2. Hazard sweep + trip-check (the mandatory step-0 gate — ledger + kill-list)

### 2a. Gap-ledger rows implicating this file / its procs
- **Row 1066 (CHARTER)** — copy/draw-bound H-crossing residual; `Draw_TileColumn`
  is plane_buffer's step-5 half. Domain split explicit. → §8.
- **Row 1064** — the H-column dossier (the fix template built as unified-prefetch);
  its *decompress* half is CLOSED. The residual is the copy/DRAW half → row 1066. Context.
- **Row 1057** — measured `Draw_TileRow_FromCache` = **11.7k** vs `Draw_TileColumn`
  = **5.4k** cyc/f-when-active ("rows cost 2× columns"). These are the JOTTED baselines;
  §8 rules on whether they suffice. Vertical-path context: t16 probe measured
  `Draw_TileRow_FromCache` 7716 (5.6%) inclusive at 16px/f vertical.
- **Row 1052 — TRIP-CHECK, MUST NOT MISCLAIM.** The VDP-macro shared-home trigger
  (2nd vdpComm consumer → shared module) does **NOT** fire here. Row 1052 was
  CORRECTED 2026-07-15: plane_buffer.asm has **zero** vdpComm/vdpCommReg uses — it
  RECEIVES precomputed VDP command longs from callers (section builds them) and
  `VInt_DrawLevel` writes **raw `$8Fxx` autoinc register words + builds the command
  longs at runtime** via the `lsl.l #2 / addq / ror.w / swap` bit-shuffle. The real
  2nd vdpComm consumer arrives with buffers.asm/dma_queue.asm/boot.asm, NOT this
  tranche. **I will not claim the shared-home trigger fires.**

### 2b. Kill-list rows this port TRIPS (const/symbol-keyed, not just file-keyed)
- **Row 5** (AS twins in lockstep) — plane_buffer.asm JOINS the list as a new
  gate-off twin, kept byte-lockstep with plane_buffer.emp until Spec 5. Standard.
- **Row 6** (per-shape gate pins) — adds the `PLANE_BUFFER` region pin + the two
  resume orgs. Standard re-pin tax.
- **Rows 7 / 8** (`Act`/`Sec` struct twins) — `Draw_BG_TileColumn` reads
  `Sec_sec_bg_layout(a0)` ($1C) and `Act_act_bg_layout(a2)` ($0E). This ADDS
  plane_buffer to the Sec/Act offset-mirror consumer set. Not a kill (structs.asm
  isn't porting); I ride the mirror the same way tile_cache_port did (file-local
  drift-locked offset value-equs + `ensure`, or adopt an Act/Sec field-access
  spelling — decided at step 2, §5). Low-stakes since proc 4 may be flagged dead (§9).

### 2c. At-next-touch rows naming plane_buffer
- **NONE.** The codename-reference audit backlog (~40 sites / 16 files) does not list
  plane_buffer (it isn't ported yet). No in-tranche at-next-touch execution owed.

---

## 3. Gate plan

**Gate symbol:** `SIGIL_EMP_PLANE_BUFFER` (mirrors `SIGIL_EMP_TILE_CACHE` /
`SIGIL_EMP_SECTION`).

**`engine.inc` edit** (`engine/engine.inc:311`, currently the unconditional
`include "engine/level/plane_buffer.asm"`), replicating the tile_cache/section pattern:
```
    ifndef SIGIL_EMP_PLANE_BUFFER
        include "engine/level/plane_buffer.asm"
    else
        ; sigil mixed build: Plane_Buffer_Reset..VInt_DrawLevel come from
        ; engine/level/plane_buffer.emp, pinned by the sigil map at the per-shape
        ; reference address. Resume placement at the region end (Tile_Cache_GetTile).
        ; Shape-INVARIANT length ($29C both shapes; no asserts / no __DEBUG__) — the
        ; two resume orgs differ only by the upstream base slide.
        ; NOTE: sonic4-shape addresses — the gate define must never be set for other games.
      ifdef __DEBUG__
        org     $4EC4
      else
        org     $42FA
      endif
    endif
```
Resume org = region END = `Tile_Cache_GetTile` = `TILE_CACHE.{plain,debug}_base`
(0x42FA / 0x4EC4). Composes cleanly with the neighbours: LOAD_OBJECT's gate resumes
at 0x405E (= our base), TILE_CACHE's gate begins at 0x42FA (= our end).

**New region pin (`pins.rs`):**
```
/// `Plane_Buffer_Reset` .. `Tile_Cache_GetTile` — gate `SIGIL_EMP_PLANE_BUFFER`. tests: plane_buffer_port
pub const PLANE_BUFFER: Region = Region { plain_base: 0x405E, debug_base: 0x4C28, plain_len: 0x29C, debug_len: 0x29C };
```
(The LOAD_OBJECT pin comment `… .. Plane_Buffer_Reset` already names our start as its end — consistent.)

---

## 4. Step-1 demanded features — **NONE**

plane_buffer is a **clean transcribe**. Unlike section (demanded `vdpComm`) or
tile_cache (demanded `coll_src_row_base`), this file invokes **no AS macro** and
needs no new language primitive. Everything it does is already in the corpus:
bare Bcc + `dbf` loops, `jbsr` (n/a — leaf), absolute-EA over link base (bare
`sym + const` form, `Tile_Cache_Nametable+TILE_CACHE_NT_SIZE` — the section.emp:303 /
tile_cache precedent), comptime immediate exprs (`VRAM_PLANE_A & $FFFF`,
`$8000 | (32-1)`), Sst-style field access for Sec/Act (step 2). ⇒ the byte gate at
step 1 has no feature dependency; it is a pure faithful port.

---

## 5. The seam (value-equs + RAM labels + struct offsets)

**Value-equs** (drift-locked mirrors the file's `ensure`s read back):
`PLANE_BUFFER_SIZE`=1536, `PLANE_H_CELLS`=64, `PLANE_V_CELLS`=64, `VRAM_PLANE_A`=$C000,
`VRAM_PLANE_B_BYTES`=$E000, `VDP_DATA`=$C00000, `VDP_CTRL`=$C00004, `TILE_CACHE_COLS`=80,
`TILE_CACHE_ROWS`=60, `TILE_CACHE_STRIDE`=80, `TILE_CACHE_NT_SIZE`=9600. Struct
offsets `Sec_sec_bg_layout`=$1C, `Act_act_bg_layout`=$0E.

**RAM address labels** (mostly already pinned by section_port/tile_cache_port):
`Plane_Buffer_Ptr` (PLANE_BUFFER_PTR ✓), `Plane_Buffer` (buffer base — `ram.asm:294`,
**pin may need adding**), `Cache_Left_Col`/`Head_Col`/`Top_Row`/`Bottom_Row`/
`Origin_Col`/`Origin_Row` (✓), `Tile_Cache_Nametable` (✓), `Section_Right_Col_Written`
(SECTION_RIGHT_COL_WRITTEN ✓), `Current_Act_Ptr` (CURRENT_ACT_PTR ✓), `VDP_DATA`/`VDP_CTRL`
(constants). Full enumeration + any missing pins = mechanical step-1 work.

**`VRAM_PLANE_B_BYTES` nuance (step-3(a) jot):** it's defined **file-local in
`engine/level/bg.asm:20`** (`= $E000`), a duplicate of `VRAM_PLANE_B` ($E000,
constants.asm:52), and bg.asm is included AFTER plane_buffer.asm — AS resolves it by
whole-program equ pass. A dedup/consolidation candidate (ledger jot at step 3); for
the .emp I mirror `= $E000` with an `ensure` (byte-lockstep is byte-level, spelling
is mine).

---

## 6. THE 3RD OWNERSHIP FLIP — cross-gate matrix + two-module link test

Per the **proof-mechanism feed-forward rule**: porting plane_buffer flips
`Draw_TileColumn` and `Draw_TileRow_FromCache` out from under **section.emp** (an
already-ported .emp module that `jbsr`s them cross-seam). This is the symbol-ownership
FLIP class → **requires a persisted two-module link test**. This is the campaign's
3rd flip (t15 entity_window = bidirectional jbsr/jsr; t16 tile_cache = tail-call jbra).

### 6a. Full caller matrix (grep-verified)
| Owned symbol | `.emp` callers (FLIP) | AS callers (normal gate) |
|---|---|---|
| `Draw_TileColumn` | **section.emp:559, 610** (`jbsr`) | section.asm:408,459 |
| `Draw_TileRow_FromCache` | **section.emp:654, 693** (`jbsr`) | section.asm:503,542 |
| `VInt_DrawLevel` | — | **vblank.asm:62** (`bsr.w`) → normal AS→.emp reverse edge |
| `Draw_BG_TileColumn` | none | none (§9) |
| `Plane_Buffer_Reset` | none | none (§9) |

⇒ the ONLY .emp-side caller is **section.emp**, via **4 `jbsr` sites** into 2 owned
symbols. UNIDIRECTIONAL (section→plane_buffer; plane_buffer is a leaf, calls nothing
back). So: mechanism = entity_window's `jbsr`/bsr.w flip; topology = tile_cache's
unidirectional single-owner shape. `VInt_DrawLevel`'s caller is AS (`vblank.asm`) —
handled by the **normal region gate** (the .emp symbol exports to the link, AS
resolves it), proven by the mixed full-build strict gate — **NOT a flip**.

### 6b. Cross-gate combos (which gate config exercises which resolution)
| SIGIL_EMP_SECTION | SIGIL_EMP_PLANE_BUFFER | section's `Draw_TileColumn` resolves to | proof |
|---|---|---|---|
| off | off | plane_buffer.**asm** (both AS) | existing full build |
| off | on  | plane_buffer.**emp** ← section.asm bsr.w | mixed full-build strict gate |
| on  | off | plane_buffer.**asm** ← section.emp jbsr | section_port synthesizes Draw_* as AS labels (unaffected by my port) |
| **on** | **on** | **plane_buffer.emp ← section.emp jbsr** | **NEW two-module link test (the flip)** |

The existing `section_port` / `entity_window_port` tests synthesize
`Draw_TileColumn`/`Draw_TileRow_FromCache` as AS-side labels at their VMAs — that
stays valid after my port (a byte gate only needs the target VMA), so **no existing
test is disrupted**; the flip proof is purely ADDITIVE.

### 6c. The persisted test (`crates/sigil-cli/tests/plane_buffer_port.rs`)
Model on `entity_window_port.rs` (jbsr mechanism) with tile_cache_port's
unidirectional single-owner union:
- **Byte gate** (both shapes): `plane_buffer_region_matches_reference` +
  `_debug_…` — compile the real `plane_buffer.emp`, place at `PLANE_BUFFER`, link
  with the value-equs + address labels, byte-compare the region. Shape-INVARIANT.
- **`negative` probe**: a doctored value-equ (e.g. wrong `PLANE_BUFFER_SIZE`) fires
  its `ensure`.
- **Flip proof** `two_module_ownership_flip_{plain,debug}`: compile
  **plane_buffer.emp + section.emp** together (section prepended with its
  `engine.constants` ambient), place each at its region, ONE `resolve_layout` + `link`
  over the union; section's label list **DROPS** `Draw_TileColumn`/
  `Draw_TileRow_FromCache` (now owned by plane_buffer.emp — no synthetic label);
  byte-compare BOTH regions. section's 4 `jbsr` bytes match the reference ONLY when
  each `jbsr→bsr.w` disp lands on plane_buffer.emp's pinned symbol VMA — the flip,
  proven per shape.

**Binding-class statement (flip proof):** LINK-TIME, both gates ON, real
plane_buffer.emp + real section.emp linked as one image. This replicates the exact
binding class of the shipped `SIGIL_EMP_SECTION=1 SIGIL_EMP_PLANE_BUFFER=1` build —
not a comptime stand-in. (Contrast the sst-usability lesson: a comptime-base probe
says nothing about link-time sites.)

---

## 7. Macro-port / typed-VDP question (deferred to step 3/4 — NOT step 1)

`VInt_DrawLevel` builds VDP command longs at **runtime** from a dynamic address
(`lsl.l #2 / addq #1 / ror.w #2 / swap`) and writes raw `$8F02`/`$8F80` autoinc
register words. Two anticipated step-3(a)/step-4 findings (flagged now, not built at
step 1 — no AS macro is invoked, so the macro-port rule does not bind at step 1):
1. the runtime address→command bit-shuffle is a comptime-fn TEMPLATE candidate (the
   `reload_anim_timer` splice class) — a `vdp_rw_cmd(areg)` that emits the 4-instr
   shuffle, drift-locked to the encoding.
2. the `$8Fxx` autoinc register-write is a candidate typed spelling (`vdp_reg(reg,val)`)
   — section's typed VDP interface covers command LONGS, not register-SET words; this
   is a possibly-new axis. Step-3(a) ask; taste-gated (must read BETTER).

Neither is demanded at step 1. Both surface as named step-3/step-4 outcomes.

---

## 8. Step-5 headline — charter row 1066, and the baseline decision

`Draw_TileColumn` is the copy/draw-bound H-crossing lever. **Decision: the existing
measured data does NOT suffice — a fresh state-counter probe is needed.** The 39-vs-24
A/B (unified-prefetch) proved DECOMPRESS prevention; it did not isolate
`Draw_TileColumn`'s exclusive cost on H-crossing lag frames, and the 5.4k figure
(row 1057) is a single-axis jot. Planned probes, each with binding class:

- **Probe A — Draw_TileColumn exclusive cost on H-crossing.** LIVE oracle profiler,
  FOREGROUND, on the SHIPPED unified-prefetch tip (`s4.debug.bin` = 827e18c4,
  hash-verified before measuring per the genesis-dev artifact rule), driven by the
  **sustained-max-horizontal** schedule (freeze `Debug_Scene_Freeze`, poke `Camera_X`
  +16px/f, advance 1 frame, `get_profiler_frames`) — the exact regime row 1066's
  residual lives in. Binding class: live per-frame profiler, real ROM, real H-crossing
  state (the state-counter method; VInt_Lag = ground truth, profiler per-frame counts
  documented-unreliable so lag-frame count is the primary metric).
- **Probe B — post-optimization A/B.** Same schedule, gate-off (twin) A vs
  optimized-.emp B, hash-verified ROMs, Frame_Counter-anchored, identical scripted
  drive (the unified-prefetch A/B protocol). Metric: H-crossing lag-frame count +
  budget%. Byte-CHANGING → re-pin + live-verify per the Wave-1 pattern.
- **Hardware cross-check (step-5 line):** `VInt_DrawLevel` is VDP-facing and runs
  INSIDE VBlank (the [[tile-cache-fill-runs-in-vblank]] lesson — a beam-position gate
  is dead here). The step-5 interrogation gets the VBlank/DMA-window awareness line +
  the **verify-during-motion** bar (mid-scroll screenshots, not at-rest) + the
  hardware cross-check line. The producers (`Draw_TileColumn`/`Row`) run in the game
  loop (not VBlank) — invariant-ladder / counter-cache / guard-coverage apply there.

Step 5 is deep in the loop; this note only STATES the plan + binding classes.

---

## 9. Dead-code flags for Volence (delete-verb (d) — FLAG, never auto-cut)

Two procs have **no callers** anywhere in aeon (`.asm`/`.emp`/`.inc`, grep-verified):
- **`Draw_BG_TileColumn`** (plane-B column strip, §4.2) — reads Sec/Act bg-layout
  ptrs; looks like **forward-scaffolding for plane-B streaming not yet wired**.
- **`Plane_Buffer_Reset`** (2-instr) — `VInt_DrawLevel` has its own `.reset`; looks
  like a **leftover/public API awaiting a consumer**.

Per the loop's delete verb: deliberate/feature/forward-scaffolding dead code → FLAG,
never auto-cut; ambiguous → treat as feature → flag. **Recommendation: KEEP + port
both faithfully** (byte-region includes them regardless), and ask Volence whether
either should be cut in a separate decision. Cutting Draw_BG_TileColumn would also
retire the Sec/Act offset seam (§2b row 7).

---

## 10. Gate-artifact list step 1 will produce (gate-artifact discipline)
- region byte gate BOTH shapes: `plane_buffer_region_matches_reference` +
  `_debug_region_matches_reference` (shape-INVARIANT).
- flip proof: `two_module_ownership_flip_{plain,debug}` (§6c).
- negative probe: doctored value-equ fires its `ensure` (name TBD at build).
- mixed-build acceptance: the full DEBUG-first + plain build with
  `SIGIL_EMP_PLANE_BUFFER=1`, gate-off neutrality (rebuild → canonical CRCs
  b335bdc6 / 827e18c4).
- gate: `SIGIL_EMP_PLANE_BUFFER` in `engine.inc` (§3); pin `PLANE_BUFFER` in `pins.rs`.
- kill-list: row for plane_buffer.asm twin (row-5 class) + region pin (row-6 class),
  same commit.
- paired-state strict gate: 194/2252/0 with `AEON_DIR` → the branch worktree.

---

## 11. Open questions for the gate (Volence / Fable)
1. **Draw_BG_TileColumn + Plane_Buffer_Reset** — keep-and-port (my recommendation) or cut? (§9)
2. **Sec/Act field access spelling** (Draw_BG_TileColumn) — file-local offset
   value-equs (tile_cache precedent) vs adopt an Act/Sec field-access idiom? (§2b/§5).
   Defer to step 2 unless Fable wants it settled now.
3. **Flip-test topology** confirmation — unidirectional section→plane_buffer, jbsr
   mechanism, section's Draw_* labels dropped (§6c). Sanity-check against the
   entity_window/tile_cache templates.
4. Anything in §7 (typed-VDP asks) Fable wants pulled forward vs left to step 3/4.

**Deliverable path:** this note → Fable's gate → step 1 (transcribe + region pin +
byte gates both shapes + the flip proof + mixed-build acceptance + negative probe +
gate-off neutrality), in the seeded worktree, branch `port-tranche17` both repos.
