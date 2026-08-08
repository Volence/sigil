# A/B evidence — widen d5 sprite-counter increments to `addq.w`

Parcel: `widen-d5-addqw` (lane-widen, parcel A). Byte-CHANGING, behavior-NEUTRAL.

## The change

Four `addq.b #1, d5` increments of the running VDP sprite counter widened to
`addq.w #1, d5`, and `out(d5: u16)` adopted on the three procs that thread it:

- `engine/objects/rings.emp:233` — `DrawRings`
- `engine/objects/sprites.emp:583`, `:589` — `Emit_ObjectPieces` (via `size_link`'s two arms)
- `engine/objects/sprites.emp:765` — `InsertSpriteMasks`

## Why it is behavior-neutral by construction

`addq.b #1, dN` and `addq.w #1, dN` are both a single 2-byte ADDQ, both 4 cycles on a
data register — the only encoding difference is the size field (`.b` = `0x5205`,
`.w` = `0x5245`). The sole observable difference between them is that `.w` also carries
into bits 8..15 when the low byte increments from `0xFF`.

That carry never happens for this counter:

- `Sprite_Render` seeds it with `moveq #0, d5`, clearing all 32 bits.
- Every increment is gated by a prior `cmpi.w #MAX_VDP_SPRITES, d5` (`rings.emp:206`,
  `sprites.emp:760`, and the loop caps in `Emit_ObjectPieces`), and `MAX_VDP_SPRITES`
  is 80 — so the low byte never reaches `0xFF` at an increment.

Therefore `d5` holds a bit-identical value after every increment under both widths, and
`move.b d5, (a4)+` (the SAT link write) is byte-identical. Every consumer — the nine
`.w` reads in `sprites.emp` (259, 277, 336, 395, 453, 475, 484, 488, 760) plus the lone
`.b` cap test at `sprites.emp:681` — reads the same value. The ROM is bit-identical in
all observable RAM/VRAM/register state at every cycle; only the 6 opcode bytes and the
header checksum change.

## Byte diff (proof the change is exactly the predicted content, no layout move)

`s4.bin` and `s4.debug.bin` each differ from their frozen golden by exactly 8 bytes:

- 6 × `0x05 -> 0x45` — the ADDQ size field flipping `.b -> .w` at the six emitted
  increment sites (DrawRings ×1, InsertSpriteMasks ×1, Emit_ObjectPieces ×4 — the two
  `size_link` arms across two flip variants each).
- 2 bytes at `0x018E..0x018F` — the Genesis header checksum, refolded over the changed
  content.

File sizes are unchanged (411429 / 423831) and `repin` reports `pins.rs unchanged`:
no ORG, region, or anchor-position moved. This is a content-only, layout-stable parcel.

## Runtime sanity (Oracle / BlastEm-class core)

Both the frozen golden `s4.bin` (`crc32 3192f989`) and the widened build
(`crc32 92e5a90f`) boot cleanly and reach `DrawRings` (`0x34DA`) — the changed code
path. At the `DrawRings` entry anchor the 68k `d5` reads `0x00000005` / `0x00000006`
across the two boots: the high 24 bits are zero, confirming at runtime the wrap-free
invariant the neutrality argument rests on.

Full-VDP-state hashes are NOT presented as an A/B pair here: the "first `DrawRings` hit"
anchor is not frame-locked (the two independent boots reached it a game-frame apart —
OLD with 6 sprites counted, NEW with 5), so their VRAM/CRAM/framebuffer differ by boot
phase, not by behavior. Per the campaign frame-lock caveat, a boot-phase-anchored hash
diff is not a behavior signal. The definitive neutrality evidence is the static
bit-identity argument above, sealed by the byte-exact 8-byte diff.
