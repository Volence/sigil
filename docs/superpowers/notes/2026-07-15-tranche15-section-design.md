# Tranche 15 — `engine/level/section.asm` step-0 recon + design

**Target:** `aeon/engine/level/section.asm` (559 lines / 21 KB) — the
continuous-scroll section-streaming engine (§4). Biggest engine file
ported to date. **Hot-path file** → step-5 gets live oracle verification
+ a Fable hot-path second look, not a paper pass.

Master at design time: sigil `feb69dc`, aeon `<fill at branch>`.
Precedent for a level-dir port: `collision_lookup.emp` (braceless
`module … in <section>`, `.asm` twin alongside, RAM trackers as
bare words with the linker asl-width comment).

## Proc inventory

| Proc | Lines | Role | Hot? |
|---|---|---|---|
| `Section_Init` | 12–19 | record act, fill NT, init entity window | init |
| `Section_FillInitial` | 33–56 | seed col/row trackers tight around camera | init |
| `Section_FlatIDXY` | 57–77 | flat id = sec_y*grid_w + sec_x (repeated-add) | cold |
| `Section_GetSecPtrXY` | 78–121 | bounds-check + ×66 stride → Sec ptr | cold |
| `Section_RedrawPlanes` | 144–335 | ~190-line atomic full-plane VDP-poke rewrite; Plane A col-major (wrap-split Part A/B) + Plane B row-major BG | **init / cache-recovery only** (never mid-traversal) |
| `Section_UpdateColumns` | 345–559 | per-frame 4-directional ring streaming: right/left cols (`Draw_TileColumn`) + bottom/top rows (`Draw_TileRow_FromCache`) | **EVERY FRAME** |

`Section_UpdateColumns` is the per-frame hot proc. Its bottom/top row
half is the **"RescanY streaming profile OPEN (OJZ domain)"** item from
the campaign memory — step-5 owes an oracle streaming profile here.

## Extern / use seam inventory (siblings NOT yet ported → `extern()`/`ensure`)

All of these live in still-`.asm` siblings; every one is a scaffolding
seam with a kill condition = "that sibling gets ported".

- **Procs** (jbsr targets): `Draw_TileColumn`, `Draw_TileRow_FromCache`
  (plane_buffer.asm), `EntityWindow_Init` (entity_window — already ported;
  resolve via its symbol).
- **RAM trackers** (bareword `.w`, collision_lookup idiom): `Camera_X`,
  `Camera_Y`, `Current_Act_Ptr`, `Section_Plane_Dirty`,
  `Section_{Right,Left}_Col_Written`, `Section_{Top,Bottom}_Row_Written`,
  `Cache_{Left,Head}_Col`, `Cache_{Top,Bottom}_Row`,
  `Cache_{Origin,Top}_Row`, `Cache_Origin_Col`, `Plane_Buffer_Ptr`,
  `Tile_Cache_Nametable`.
- **Consts** (`engine/constants.asm`): `PLANE_V_CELLS`=64, `PLANE_H_CELLS`,
  `PLANE_BUFFER_SIZE`, `SECTION_SIZE_SHIFT`=11, `TILE_CACHE_ROWS`=60,
  `TILE_CACHE_COLS`, `TILE_CACHE_STRIDE`, `TILE_CACHE_NT_SIZE`,
  `VRAM_PLANE_A`, `VRAM_PLANE_B_BYTES`, `VRAM_PLANE_A_BYTES`.
- **Struct fields** (`engine/structs.asm`): `Act_grid_w`, `Act_grid_h`,
  `Act_sec_grid_ptr`, `Act_act_bg_layout`, `Sec_sec_bg_layout`, `Sec_len`.

## Structural pins (step-1 transliterations, each with a site comment)

1. **`Sec_len <> 66` stride assert** (GetSecPtrXY, lines 103–105): the
   `lsl #6 + lsl #1 = ×66` stride trick. **This exact assert recurs in
   `tile_cache.asm:189`** — a corpus-sweep signal (see below). Structural
   `ensure(Sec_len == 66)` + pinned shifts.
2. **`vdpCommReg d2, VRAM, WRITE, 1`** macro (RedrawPlanes ×2) — macro
   transliteration, keep spelling identical to the twin.
3. **`vdpComm(VRAM_PLANE_B_BYTES,VRAM,WRITE)`** comptime expr (line 314).
4. **`$8F80` / `$8F02` / `$2700`** VDP register + SR literals — magic-number
   audit in step-3(b); step-1 keeps them literal with carried comments.

## Struct-twin adoption question (the flagged step-4 candidate)

`section.asm` reads the `Sec` and `Act` structs via `Sec_*` / `Act_*`
field-offset consts — the same shape EntityScanState/sst were adopted
into via struct-twin + `offsetof`/`sizeof`. **Difference from
EntityScanState:** `Sec` and `Act` are **multi-consumer** (section.asm,
tile_cache.asm, camera.asm, parallax.asm all use them). EntityScanState
went file-local precisely because it was single-consumer; the ratified
complement is *"a shared struct earns a module"* (the sst.emp precedent).

**Decision (deferred to step-4, consumer-gated):** transcribe step-1
with the `Act_*`/`Sec_*` field consts mirrored as drift-locked consts
(pre-adoption entity_window shape). The `Sec`/`Act` struct-twin belongs
in a **shared `engine.structs`-style module**, not file-local — but that
module's first mover shouldn't be section.asm alone when tile_cache /
camera / parallax are still `.asm`. Flag as a **campaign-level struct
module ask** (ledger), adopt when a second struct consumer ports or when
Volence wants the shared module stood up. Do NOT hand-build a file-local
`Sec` struct that a later shared module would have to unwind.

## FIRST VDP-command-macro consumer (step-1 demanded-feature build)

`section.asm` is the **first `.emp` file to use the VDP command macros** —
no prior port touched them. Two must ship at step-1 (demanded-features law):

1. **`vdpComm(addr, type, rwd)`** — AS `function` computing a VDP command
   longword (`macros.asm:8`). Pure value → model on `objdef.emp`'s
   `vram_art` (`pub comptime fn … -> int`). Used at line 314:
   `move.l #vdpComm(VRAM_PLANE_B_BYTES,VRAM,WRITE), (a5)`.
2. **`vdpCommReg reg, type, rwd, clr`** — AS `macro` (`macros.asm:264`)
   emitting an in-place register→command instruction sequence with FOUR
   comptime guards (`(type&rwd)&3`, `clr`, `(type&rwd)&$FC == $20`, else).
   → `comptime fn vdp_comm_reg(reg: Reg, …) -> Code` — model on
   `animate.emp`'s `reload_anim_timer(src: Reg)` for the `{reg}` splice +
   `sprites.emp`'s `y_term` for the comptime-if branch emission. Composed
   via `++` concat over conditionally-included `asm{}`/`asm{}`-empty
   fragments (no `let mut` in the corpus; `fold`/branch-return only). Used
   ×2 in RedrawPlanes: `vdpCommReg d2, VRAM, WRITE, 1`.

**VDP type consts** the macros consume (`constants.asm:36-41`):
`VRAM=%100001 CRAM=%101011 VSRAM=%100101 READ=%001100 WRITE=%000111
DMA=%100111`. Mirror locally + drift-lock (entity_window pattern).

**HOME decision (both macros + type consts): file-local in section.emp,
ledger the hoist.** These are shared engine infra (macros.asm/constants.asm
serve the whole engine), but the ratified port discipline is
byte-isolation — entity_window mirrored engine consts locally and ledgered
"step-4 candidate to hoist into constants.emp" rather than reaching into a
shared file mid-port. Same call here: build `vdpComm`/`vdp_comm_reg`
file-local (they're comptime → zero bytes, a pure later move), ledger a
**shared `engine.macros.emp` / `engine.vdp` home ask** to adopt when a 2nd
VDP-macro consumer ports (plane_buffer, tile_cache, load_art all use them).
Symmetric with the Sec/Act struct-twin call above.

## Const values (mirror + drift-lock targets)

| Const | Value | Truth file |
|---|---|---|
| `VRAM_PLANE_A` | `$C000` | constants.asm:51 |
| `VRAM_PLANE_B_BYTES` | `$E000` | **bg.asm:20** (not constants.asm) |
| `SECTION_SIZE_SHIFT` | 11 | constants.asm:305 |
| `PLANE_H_CELLS` / `PLANE_V_CELLS` | 64 / 64 | constants.asm:77-78 |
| `PLANE_BUFFER_SIZE` | 1536 | constants.asm:335 |
| `TILE_CACHE_COLS` / `ROWS` | 80 / 60 | constants.asm:340-341 |
| `TILE_CACHE_STRIDE` | `= TILE_CACHE_COLS` | constants.asm:342 |
| `TILE_CACHE_NT_SIZE` | `COLS*ROWS*2` = 9600 | constants.asm:343 |
| `Sec_len` / `Sec_sec_bg_layout` | `$42` / `$1C` | structs.asm |
| `Act_sec_grid_ptr` / `_grid_w` / `_grid_h` / `_act_bg_layout` | `$00`/`$04`/`$06`/`$0E` | structs.asm |

## Design decisions (step-1)

- **Module form:** `module engine.section in section` (braceless,
  collision_lookup precedent; procs at col 0, classic asm indent inside).
- **RAM trackers:** bare words with the one-time linker asl-width comment
  (`(X).w — ram.asm; abs.w picked by the linker's asl width rule`), not
  `.w`-suffixed — matches collision_lookup and the row-1010 finding that
  the `.w` operand-override does NOT compile for link-time bases.
- **`.w` branches** (`bra.w .right_loop`, `blt.w .pla_next`, etc.): keep
  explicit-width at step-1 (transcribe faithful), convert to bare Bcc /
  `jbra` at **step-2** (shrink twin in lockstep + re-pin per the loop).
- **Contracts:** each proc's In/Out/Clobbers header → `clobbers()`/`out()`.
  Note `RedrawPlanes` clobbers d0–d7/a0–a6 (full set) and `UpdateColumns`
  d0–d7/a0–a3 (movem-saved d2-d7/a0-a3, so d0/d1 the extra clobbers).

## Harness / gate plan

Mirror the collision_lookup / entity_window wiring:
- New gate env `SIGIL_EMP_SECTION`; byte-gate test `section_port`.
- Region pin `Section_Init .. Section_UpdateColumns` in `pins.rs`.
- Byte gate BOTH shapes (plain + debug) + mixed-build acceptance +
  gate-off neutrality + negative probe.
- Paired-state gate: full strict suite green with `AEON_DIR` → the
  **branch** tree before merge, never aeon master.

## Corpus-sweep flags seeded this tranche (step-6 obligations)

- The `Sec_len <> 66` stride assert is shared with `tile_cache.asm` — if
  a `Sec` struct-twin / stride helper ships, tile_cache is a sweep site.
- Any `Draw_TileColumn`/`Draw_TileRow_FromCache` extern-seam idiom used
  here is reusable when plane_buffer.asm ports.

## Language-ask preview (step-3(a), confirm during port)

- **`asr` by a `moveq`-loaded shift const** (`moveq #SECTION_SIZE_SHIFT,d0
  / asr.w d0,d2`, lines 299–302) — a variable-shift idiom; check whether
  the language wants a named shift form.
- Watch the four near-identical **clamp ladders** in `UpdateColumns`
  (right/left/bot/top each do cache-clamp + wrap-clamp + tracker-min/max):
  a structural-clone / `emit_*` construct candidate for step-4.
