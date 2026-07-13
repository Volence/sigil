# Tranche 13 — load_object.asm — Step 0 design note (2026-07-13)

Target: `aeon/engine/objects/load_object.asm` (107 lines, 2 procs:
`Load_Object` + `Load_ObjectList`). Ratified t13 (Volence). Starts on
post-retro-fix-batch-2 master (aeon `668085d` / sigil `34883c6`).

## Region bounds (master, both shapes)

| Shape | `Load_Object` | `Plane_Buffer_Reset` (region end) | Length |
|-------|--------------|-----------------------------------|--------|
| plain | `$3FDC`      | `$407A`                           | `$9E` (158 B) |
| debug | `$4BA6`      | `$4C44`                           | `$9E` (158 B) |

Region = `[Load_Object, Plane_Buffer_Reset)` — `Plane_Buffer_Reset` is the
first label of `plane_buffer.asm`, the include immediately after
load_object.asm (engine.inc:295).

**SHAPE-INVARIANT LENGTH** — no `assert`, no `ifdef __DEBUG__` in the file,
so both shapes are `$9E`. The two shapes are byte-identical to each other
too (only relative branches + register-indirect + a `bsr.w AllocDynamic`
link ref; zero absolute operands). Simplest region shape since dplc.
Still pinned per-shape (base differs by the children.asm-and-upstream slide).

## Hazard sweep (gap-ledger + retro-audit)

- **AllocDynamic / A2-latch coupling** — `Load_Object` calls `AllocDynamic`
  (now A2-overflow-latched on post-merge master). It is a VERIFIED-CLEAN
  compact-on-full caller: it writes `SST_code_addr(a1)` IMMEDIATELY after a
  successful alloc (load_object.asm:35), satisfying AllocDynamic's
  undocumented "set code_addr immediately" caller invariant (named clean in
  2026-07-12-steps2-5-retro-audit.md:244). No hazard action; the deliverable
  is a step-3(b) contract note documenting the invariant reliance.
- No other file-implicating ledger rows for load_object / Load_Object /
  Load_ObjectList.

## Design decisions

1. **ObjDef struct-twin: NOT built in this port (ratification rationale
   revised — surface at merge).** The t13 ratification predicted an ObjDef
   struct-twin + that this file would drive the `offsetof` gap. Reading the
   real code: `Load_Object` burst-copies the 26-byte template as an OPAQUE
   byte block (`movem.l (a2)+, d3-d4` ×3 → `movem.l d3-d4, N(a3)`) and never
   names an ObjDef field. All structured writes are to the SST via
   register-displacement (`SST_x_vel(a1)` …), fully covered by the existing
   `sst.emp` twin (`Sst.field(a1)`). There is NO absolute-address struct EA,
   so `offsetof` is NOT demanded here. The ObjDef struct-twin's real driver
   is the DATA file that EMITS objdefs (the `objdef` macro, macros.asm:82 /
   `data/objdefs/test_objects.asm`), not this consumer — matching the
   tranche-6 design's own note ("ObjDef as a typed .emp struct … NOT demanded
   until an ObjDef consumer file ports"). The `offsetof` gap stays open,
   correctly deferred to that data-file tranche. → step-3(a) ledger row.

2. **Constants** — `RF_XFLIP`, `RF_YFLIP`, `FRAME_PIECE_COUNT` all live in
   `engine.constants` (`engine/system/constants.emp`) already, drift-locked.
   `use` them. `OEF_XFLIP`/`OEF_YFLIP` appear only in comments (the flip
   mapping is a literal `rol.w #4`), so no symbol needed.

3. **SST field access** — `sst.emp`'s typed `Sst.field(a1)` displacements
   throughout (code_addr / x_vel / x_pos / y_pos / subtype / render_flags /
   status / prev_anim / prev_frame / mappings / sprite_piece_count).

4. **Externs** — `AllocDynamic` (link symbol, `bsr.w`). That's the only one.

5. **Seam** — `entity_window.emp` already calls `jbsr Load_Object` as a bare
   link ref (entity_window.emp:1199). Porting closes the spawn seam
   .emp↔.emp with NO seam edit. `children.asm` (still .asm) calls
   AllocDynamic but not Load_Object; unaffected.

6. **Gate** — add `SIGIL_EMP_LOAD_OBJECT` at engine.inc:294 (today an
   ungated unconditional `include`). Region base sits below children.asm
   (unported, fixed size) → stable base. Resume org at region end
   (plain `$407A` / debug `$4C44` before any step-2 size change).

## Step-1 transcription inventory (faithful, widths carried)

- `movem.l d0-d2/a1,-(sp)` / `movem.l (sp)+,d0-d2/a2` — reglist save/restore.
- `bsr.w AllocDynamic` (→ `jsr` in the .asm; keep `bsr.w`/faithful at step 1,
  jbsr at step 2). NB the .asm spells it `jsr AllocDynamic` + `bne.w .alloc_fail`.
- movem burst copy ×3 (opaque template block).
- `swap`/`clr.w`/`move.l` position build ×2; `rol.w #4` flip map; masked `or.b`.
- `move.l #$FF000000, SST_prev_anim(a1)` — long immediate runtime init.
- mappings→frame-0 piece-count read: `movea.l`, `move.w (a3)`,
  `move.w FRAME_PIECE_COUNT(a3,d3.w), d3` (indexed displacement).
- `Load_ObjectList`: `movea.l (a0)+,a1` list walk, `jsr Load_Object`, `bra.s .loop`.

Branch widths present (become bare/jbra/jbsr at step 2): `bne.w .alloc_fail`,
`beq.s .no_piece_count`, `bra.s .loop`, `beq.s .done`.

## Harness plan

- New `[[region]] load_object` in `repin.toml` (after entity_window's
  neighbors), symbol pins: `Load_Object`, `Load_ObjectList` (+ any others the
  gate needs). Gate `SIGIL_EMP_LOAD_OBJECT`.
- Byte gates both shapes: `load_object_port.rs` (plain `s4.bin[$3FDC..$407A]`,
  debug `s4.debug.bin[$4BA6..$4C44]`).
- Mixed-build acceptance: the entity_window↔load_object spawn seam (both .emp)
  + a synthetic cross-seam consumer if warranted.
- Negative probe + gate-off neutrality (include path unchanged when gate unset).
