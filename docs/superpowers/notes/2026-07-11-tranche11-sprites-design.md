# Tranche 11 — engine/objects/sprites.asm port design (Step 0)

Written 2026-07-11 on sigil branch `port-tranche11` / aeon worktree
`sigil-emp-tranche11`. Baseline verified: `repin --check` = "pins.rs
unchanged"; master pins plain `15f2d69e…` / debug `2d095a44…` (tranche-10
provenance). This is the first port against the FINISHED language (Spec 2
frozen; C1/C2/out/clear_longs/table all merged).

## The file

`engine/objects/sprites.asm`, 648 lines. Priority-banded display-list
builder + VDP SAT (Sprite Attribute Table) emitter. Seven symbols:

| Symbol | Role | Contract |
|---|---|---|
| `InitSpriteSystem` | frame-start: clear band counts + counters | clobbers d0,a0 |
| `Draw_Sprite` | cull an object, add to its priority band (with cascade) | **preserves a0,d7** (RunObjects_Frozen loop invariant); clobbers d0-d3,a1 |
| `Render_Sprites` | walk bands 7→0, emit SAT, insert masks, DrawRings | clobbers d0-d7,a0-a6 |
| `CellOffsets_XFlip` | 16-byte `dc.b` flip-width LUT + `align 2` (data in code region) | pc-relative read |
| `Emit_ObjectPieces` | emit one object's pieces to SAT — **4 near-identical flip variants inline** | detailed (see source header) |
| `InsertSpriteMasks` | write X=0 mask sprites into SAT | clobbers d0-d1 |

## Addresses (from s4.lst / s4.debug.lst, dated 11 Jul 16:49)

- Region: `InitSpriteSystem` → `AnimateSprite` (animate region's start).
- **Plain**: $2954 → $2D74. **Debug**: $2C0E → $302E.
- **Length $420 BOTH shapes.** sprites.asm has NO `ifdef`/`__DEBUG__` —
  shape-INDEPENDENT, base-shifted only (core is longer in debug, so
  sprites' base slides $2954→$2C0E, but its own length is constant). This
  is the common same-length-both-shapes case (unlike core/rings).
- Downstream slide: AnimateSprite's region base ($2D74/$302E) and every
  region after it move by whatever step-2 shrink sprites accrues; `org
  $10000` absorbs the total as pad (tranche-10 shield), so EndOfRom +
  object-bank/data stay UNCHANGED and only engine-downstream re-pins.

## Gate — D-T11.1

Single `SIGIL_EMP_SPRITES` region. sprites.asm is currently UNGATED (plain
`include` at engine.inc:206); the port adds the gate scaffold (resume `org`
per shape after the include, mirroring the core/animate blocks) + a new
`[[region]]` in repin.toml (`start=InitSpriteSystem end=AnimateSprite
gate=SIGIL_EMP_SPRITES`), inserted between the `core` and `animate` regions
in ladder order.

## Constants & twins — D-T11.2 (FORCED row-17 flip)

sprites.asm DEFINES three consts at lines 6-8 and USES them:
`VDP_SPRITE_Y_OFFSET=128`, `VDP_SPRITE_X_OFFSET=128`, `MAX_VDP_SPRITES=80`.
It also defines two module-local consts (`SPRITE_MASK_SIZE=%00000011`,
`SPRITE_MASK_HEIGHT=32`) with NO other consumer — those stay sprites.emp
module-local (no twin, "NOT on the list" class).

The three geometry consts are **kill-list row 17**: mirrored in
constants.emp (`pub const`, lines 80-82) with 3 ensures vs
`extern("…")`. Consumers found in recon:
- `.emp`: **rings.emp** `use engine.constants.{MAX_VDP_SPRITES,
  VDP_SPRITE_X_OFFSET, VDP_SPRITE_Y_OFFSET}` (rings.emp:13) — comptime.
- AS: **rings.asm** (the gate-off rings twin) reads all three as
  `cmpi.w #MAX_VDP_SPRITES` / `addi.w #VDP_SPRITE_*_OFFSET` — IMMEDIATES.

**The flip is forced, not optional.** When `SIGIL_EMP_SPRITES` is ON,
sprites.asm is excluded, so its AS definitions vanish. But rings.asm
(gate-off, when `SIGIL_EMP_RINGS` is off) still needs `MAX_VDP_SPRITES` in
an immediate. The reverse-seam equ export (D2.34) can't serve an immediate
position — that's the `.b`/`.w` imm-link deferral, explicitly unshipped
(kill-list rows 4/10/18). So the only byte-neutral resolution available now
is row 17's OWN alternate kill: **hoist the three `=` definitions
sprites.asm → engine/constants.asm** (an always-included AS file). Then:
- AS: constants.asm defines them → rings.asm + gate-off sprites.asm both
  resolve. Remove the defs from sprites.asm (I own the twin).
- `.emp`: constants.emp keeps the `pub const` mirror (now truly mirroring
  constants.asm, **row-1 class**); ensures unchanged (extern → constants.asm).
  sprites.emp + rings.emp read via `use engine.constants` (rings.emp already
  does; sprites.emp adds the import instead of defining them).
- Row 17 collapses into row 1's condition (constants.asm ports → flip).

This is byte-neutral (same values, same immediates, relocated definition)
and arguably reads-better (VDP sprite geometry is engine-wide hardware fact,
not sprites-module-private). Because the build cannot assemble without it,
the hoist ships in **step 1** (demanded-structural-change law) and the
kill-list row-17 rewrite lands in **step 3**.

Constants sprites.emp consumes via `use engine.constants`: PRIORITY_BANDS,
SPRITES_PER_BAND, SCANLINE_BANDS, SCANLINE_SPRITE_LIMIT, SPRITES_PER_BAND,
SCREEN_WIDTH, SCREEN_HEIGHT, RF_ONSCREEN, RF_XFLIP, RF_YFLIP, RF_COORDMODE,
RF_MULTISPRITE, RF_PRIORITY_SHIFT, FRAME_BBOX_X_MIN/MAX, FRAME_BBOX_Y_MIN/MAX,
FRAME_PIECE_COUNT, FRAME_PIECES (all already in constants.emp) + the three
hoisted geometry consts.

SST fields used (all in sst.emp, row 11): parent_ptr, render_flags,
mappings, mapping_frame, x_pos, y_pos, art_tile, sprite_piece_count,
sibling_ptr. `use engine.objects.sst.{Sst}`.

## Cross-module reference surface — D-T11.3

- **core.emp → sprites.emp**: `core.emp:352 jbsr Draw_Sprite` (inside
  RunObjects_Frozen). Bare branch target → resolves through the flat link
  symbol table to sprites.emp's `pub proc Draw_Sprite`. No `use` (that's for
  comptime values/types). Gate-independent: gate-off, Draw_Sprite comes from
  sprites.asm; gated, from sprites.emp — both link fine.
- **sprites.emp → rings.emp**: `bsr.w DrawRings` → `jbsr DrawRings`
  resolves to rings.emp's `pub proc DrawRings`. Same flat-link story.
- **AS games → sprites.emp**: `jsr Render_Sprites` / `jsr InitSpriteSystem`
  in demo_state / object_test_state / ojz_scroll_test resolve to the .emp
  exports (standard reverse seam — the exported proc label).

## RAM labels (engine/ram.asm, `.w`-addressable, extern/absolute)

Sprite_Table_Buffer, Sprite_Table_Dirty, Sprite_Bands, Sprite_Band_Counts,
Sprites_Rendered, Sprite_Cycle_Counter, SpriteMask_Y/Height/After_Band,
Scanline_Band_Sprites, Camera_X, Camera_Y. All `(Label).w` absolute reads —
the rings/collision RAM-label idiom.

## Step-4 construct candidate — D-T11.6 (the headline)

`Emit_ObjectPieces` holds FOUR flip-variant piece loops (unflipped / xflip /
yflip / xyflip), ~35-40 lines each, dispatched by a masked flip byte. They
share the SAT-write skeleton (Y at +0, size+link at +2/+3, tile at +4, X at
+6, `dbeq d4` cap) but differ in the geometric transform:
- unflipped: Y+=off; X+=off
- xflip: tile ^= $0800; X = -off - width(size) + …
- yflip: tile ^= $1000; Y = -off - height(size); size re-read
- xyflip: tile ^= $1800; both transforms

This is the tranche's marquee step-4 question: does a comptime-fn
(`emit_piece_loop(flip_flags)`) that parametrizes the transform earn its
keep (BUILD), or is it big enough to be a language ASK (a `flipvariants`
construct), or do the four stay inline (the zero-JSR-per-piece perf reason
they're unrolled today = a legitimate "not taken, logged" step-5 outcome)?
**Decide at step 4 with the code in front of me, not now.** The
`CellOffsets_XFlip` LUT (D-T11.5) is consumed only by the xflip/xyflip
variants — its fate is tied to this decision.

`CellOffsets_XFlip` (16 `dc.b` + `align 2`) is data embedded in the code
region, read pc-relative. Step-1 spelling: `data`/`dc.b` in the module;
watch the `align 2` (odd-length 16 is already even — align is a no-op guard,
but transcribe it). Possible `table`/`offsets` construct touch at step 4.

## Step-5 target — D-T11.7

`Render_Sprites` + `Emit_ObjectPieces` are the hottest render code: per
piece, per frame, up to 80 pieces (MAX_VDP_SPRITES). The link-order cycling
(reverse intra-band on odd frames), scanline-band budget, and multi-sprite
sibling walk are all live-behavioral — any change needs oracle verification
(Prof_RenderSprites RAM counter already exists; object_test_state profiles
Render_Sprites). "No changes, recorded why" is a valid outcome given the
code is already hand-unrolled for cycles.

## Plan of record

1. **Step 1** transcribe sprites.emp 1-1 (incl. the D-T11.2 hoist as a
   demanded structural change) + gate scaffold + repin.toml region +
   sprites_port.rs (byte gate both shapes + mixed-build + gate-off
   neutrality + negative probe). Byte-exact $420 both shapes.
2. **Step 2** modernize: bare Bcc / jbra / jbsr / Sst.field, twin lockstep,
   re-pin wave, gates re-green.
3. **Step 3** retrospect: row-17 rewrite, language/format asks, reads-wrong.
4. **Step 4** construct pass: the Emit_ObjectPieces four-variant decision.
5. **Step 5** optimize: oracle-profile Render_Sprites; take or log.
6. Loop until dry → **step 6** corpus sweep → packet → Volence gate → merge.
</content>
</invoke>
