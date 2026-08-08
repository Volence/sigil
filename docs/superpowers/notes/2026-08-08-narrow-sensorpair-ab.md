# A/B evidence — narrow `Player_SensorPair`'s d1/d2 stack round-trip to `.b`

Parcel: `narrow-sensorpair` (lane-cfg, parcel 2). Byte-CHANGING, size-neutral,
behavior-NEUTRAL. Discharges the parcel-1 `out(d1: u8, d2: u8)` adoption kill
condition (campaign-gap-ledger.md, "Player_SensorPair's d1/d2 stack round-trip").

## The change

`Player_SensorPair` (`games/sonic4/player/player_sensors.emp:241`) saves probe
A's result across probe B on the stack. The ANGLE (d1) and ATTR (d2) results are
byte values (`out(d1: u8, d2: u8)`, adopted parcel 1); only DIST (d0/d5) is a
word. Four instructions narrow `.w -> .b`:

- push `move.w d1,-(sp)` @246 -> `move.b` (A angle)
- push `move.w d2,-(sp)` @247 -> `move.b` (A attr)
- pop  `move.w (sp)+,d3` @251 -> `move.b` (A attr, popped into d3)
- pop  `move.w (sp)+,d4` @252 -> `move.b` (A angle, popped into d4)

The DIST push @245 (`move.w d0,-(sp)`) and pop @253 (`move.w (sp)+,d5`) stay
`.w` — d5 is read at word width (`cmp.w d0,d5` @254, `move.w d5,d0` @256).

## Why it is behavior-neutral by construction

**Byte-lane pairing.** On the 68000, `move.b Dn,-(a7)` predecrements a7 by 2
(the A7 special case keeps the stack word-aligned) and writes Dn[0..7] to the
slot's HIGH byte (the even address a7 lands on); `move.b (a7)+,Dn` reads that
same high byte back into Dn[0..7] and post-increments a7 by 2. A byte push
matched with a byte pop therefore transfers `Dn_src[0..7] -> Dn_dst[0..7]` — the
register's LOW byte survives the round-trip. The push order (d0.w, d1.b, d2.b)
and LIFO pop order (d3.b, d4.b, d5.w) keep every pair on its own 2-byte slot:
d2.b<->d3.b, d1.b<->d4.b, d0.w<->d5.w.

**Only the low byte was ever observable.** In the pre-narrowing code the popped
d3/d4 held full words, but their ONLY consumers are `move.b d3,d2` @258 and
`move.b d4,d1` @257 — both read the LOW byte (d3[0..7] / d4[0..7]), which the
old `move.w` round-trip had filled from the pushed d2[0..7] / d1[0..7]. So the
observable output (d1=angle, d2=attr on the A-wins path) already depended only on
those low bytes. Under the narrowing the low bytes survive identically, so the
outputs are bit-identical. On the `.b_wins` path d3/d4 are not consumed at all
(d0/d1/d2 hold probe B's result). d5 (dist) is untouched by the change.

**d3/d4 are clobbered temporaries, not outputs.** `Player_SensorPair` declares
`clobbers(d0-d5/a1)`; its results are d0/d1/d2. The bits 8..31 of d3/d4 that a
`.b` pop now leaves stale are read by nothing before `rts` and belong to no
caller — enumerated below.

**More correct, not merely equal.** The pushed high bytes the old code carried
were UNDEFINED by contract (d1/d2 are `u8` results of the probe, high bytes
unspecified). `move.b` pushes only the defined byte; the narrowing stops storing
and reloading a garbage byte that was never consumed.

**Stack balance.** A byte push/pop on a7 still moves SP by 2, so the three
pushes and three pops stay balanced exactly as before.

## Downstream consumers of the `.b`-popped d3/d4 (the two-sided bar)

Every read of the narrowed pop destinations, from pop to `rts`:

- `d3` (popped `.b` @251): read ONLY at `move.b d3, d2` @258 — `.b`.
- `d4` (popped `.b` @252): read ONLY at `move.b d4, d1` @257 — `.b`.
- No other read of d3/d4 exists in the body; both are in `clobbers(d0-d5)`, so no
  caller relies on them. `d5` (dist) stays `.w` and is read `.w` (@254, @256).

Every consumer reads `.b` only — the two-sided narrowing bar passes. Had any read
wider, the narrowing would leave stale high bits and this parcel would STOP.

## Cycles (same length, same timing)

`move.b Dn,-(An)` and `move.w Dn,-(An)` are both a single 2-byte MOVE at 8 cycles
on the 68000 (only `.l` differs, at 12). `move.b (An)+,Dn` and `move.w (An)+,Dn`
are both 8 cycles (`.l` = 12). All four narrowed sites are cycle-neutral as well
as length-neutral; only the size field of the opcode changes.

## Byte diff (proof: exactly the predicted content, no layout move)

`s4.bin` and `s4.debug.bin` each differ from their frozen chain-54 golden by
exactly 5 bytes — 4 opcode bytes plus the header checksum:

| shape | site | old -> new | instruction |
|---|---|---|---|
| s4 | 0x005610 | `3F -> 1F` | `move.w d1,-(sp)` -> `.b` |
| s4 | 0x005612 | `3F -> 1F` | `move.w d2,-(sp)` -> `.b` |
| s4 | 0x00561A | `36 -> 16` | `move.w (sp)+,d3` -> `.b` |
| s4 | 0x00561C | `38 -> 18` | `move.w (sp)+,d4` -> `.b` |
| s4 | 0x00018E | `56 -> D6` | Genesis header checksum |
| s4.debug | 0x0064E0 | `3F -> 1F` | `move.w d1,-(sp)` -> `.b` |
| s4.debug | 0x0064E2 | `3F -> 1F` | `move.w d2,-(sp)` -> `.b` |
| s4.debug | 0x0064EA | `36 -> 16` | `move.w (sp)+,d3` -> `.b` |
| s4.debug | 0x0064EC | `38 -> 18` | `move.w (sp)+,d4` -> `.b` |
| s4.debug | 0x00018E | `9C -> 1C` | Genesis header checksum |

`Player_SensorPair` is a single-emission proc (not a comptime template), so it
contributes 4 opcode bytes PER SHAPE. Emitted-site counts by shape:
s4 4, s4.debug 4, config_a 4, config_b 4, lean 4, demo 0, demo.debug 0 — the demo
game does not include the sonic4 player module (verified: demo goldens unchanged
by the re-freeze).

File sizes are unchanged (411429 / 423831); `repin` reports `pins.rs unchanged`
— no ORG, region, or anchor moved. Content-only, layout-stable.

## No frame-locked A/B

Player_SensorPair runs inside the per-frame player collision probe, reached only
under live gameplay with a player on terrain; the two boots would reach it a game
frame apart, so a boot-phase-anchored VDP/state hash would differ by phase, not
behavior (the campaign frame-lock caveat). The definitive neutrality evidence is
the static byte-lane-pairing argument above, sealed by the byte-exact 5-byte
diff: no register or memory state observable to any consumer changes at any cycle.
