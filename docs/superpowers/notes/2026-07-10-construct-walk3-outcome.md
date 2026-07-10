# Construct-walk #3 outcome — the Sonic domain types (2026-07-10, Volence driving)

Run in the 6/7 gap as ratified. Read against the real demand evidence:
collision.asm's SST traffic (y_pos ×7 / x_pos ×5 / velocities / AABB
radii), player_ground's GetSineCosine + slope projection (cos·gsp>>8),
player_common's ground_speed overlay field.

## Ratified vocabulary (aeon `engine/system/types.emp`, zero-byte module)

| Type | Repr | Applied to | Naming rationale (Volence's calls) |
|---|---|---|---|
| `Coord` | fixed<16,16> | x_pos/y_pos | NOT "SubPixel" — that word already means the LOW WORD here; NOT "WorldPos" — RF_COORDMODE carries screen coords in the same field. Space-neutral format+role. |
| `Velocity` | fixed<8,8> | x_vel/y_vel (+ ground_speed when player ports) | NOT "Speed" — signed/directed (physics-correct) and avoids blurring the ground_speed FIELD name. |
| `Angle` | u8 | angle; GetSineCosine's d0 param | byte-angle, $100 = full turn |
| `ObjRoutine` | u16 | code_addr | the walk's NEW find (tranche 6's dispatch word — not in the original brainstorm) |
| `Radius` | u8 | width/height_pixels | hitbox half-extent domain |
| `VramArtTile` | u16 | art_tile | the PACKED word (vram_art's shape); index/×32 conversion family waits for the VRAM port |
| `AnimId` / `FrameId` | u8 | anim, prev_anim / mapping_frame, prev_frame | anim_frame stays RAW u8 deliberately — a script CURSOR, not a frame identity |

## Rulings beyond naming

- **sin/cos output is NOT a Velocity** — it is the bare fixed<8,8> unit
  fraction (sin·$100); it becomes a velocity only ×speed. Volence probed
  this ("Angle in, Velocity out?") and the distinction held; recorded in
  math.emp's contract comment and the ledger (with the out-typing ask —
  register outputs can't carry annotations yet).
- **Refinements deferred** (`where` bounds are implemented but nothing in
  current consumers demands one; VramTile's 0..2047 arrives with the VRAM
  port).
- Deferred to their subsystems: MusicId/SfxId, PaletteLine/Color9,
  Tile/Block/Chunk, Height, Frames/Pixels, dimensional types (S2-D8).

## Application (byte-neutral — types erase; every gate green unchanged)

types.emp NEW; sst.emp retyped (annotation diff); GetSineCosine gained
`(d0: Angle)` (the tranche-3 deferred item, closed); ambient wiring for
the new `use` edges (test_objects_port, tranche6 probes, math_port, mixed
harness incl. a `types_ambient_items` helper + math arm). Strict 1998/0.
