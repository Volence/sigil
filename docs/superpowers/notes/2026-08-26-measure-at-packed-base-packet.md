# BGROOM-3 — measure at the packed base (packet)

Branch `fix/measure-at-packed-base` (from master a0fbee24). Aeon source: `.aeon-landing`
at 058ad606 (ROM CRCs verified before any work: s4 875d591f/699223, s4.debug
a02d36db/715114, demo bf2cdb42/96412, demo.debug 62a0019e/101120 — the provenance tip).

## 1. Reproduction (step 1) — CONFIRMED, mechanism RE-DIAGNOSED

Shadow COPY of `.aeon-landing` (rsync, no `.git`, no top-level ROMs), seven `nop`s
injected at the top of `RingCollision` (`engine/objects/rings.emp`), then the same
entry `sigil build` uses (`sigil build --aeon <copy> --native --game sonic4`, i.e.
`native::build_rom_chained_with_listing` on the sonic4 profile):

- clean copy: `crc=875d591f len=699223` (identity), wall 1.20 s
- grown copy, exact failure:

```
error: native build (sonic4 plain): packed layout overlaps at its real bases — a run grew
into a declared anchor; the anchor is hardware-fixed, so this content does not fit and
needs a hand ruling (the map's re-layout parcel, or less content). span pass:
resolve_layout: 1 diag(s); first Some(Diagnostic { level: Error, message: "sections
`section\036` [0x5D3C, 0x6168) and `player_sensors\050` [0x5860, 0x5D54) overlap in the
image (colliding pins)", ... })
```

Numbers (from a temporary trace of `packed_true_bases`, not committed):

| quantity | value |
|---|---|
| `player_sensors` frozen provisional base | 0x5840 |
| packed base after +14 B upstream | 0x5860 (+0x20: growth rounded to its align-16) |
| length every MEASURING round reported (`img`, `img2`, rounds 0 and 1) | **0x4DC** |
| length at the packed base (the checked resolve's extent 0x5860..0x5D54) | **0x4F4** |
| difference | 0x18 = 24 B = 12 sites × 2 B |
| differing sites | the 12 `lea` in `games/sonic4/player/player_sensors.emp` (lines 202 `SolidityTable`, 208 `AngleTable`, 214 `{ptable}` = HeightMaps/HeightMapsRot, in `probe_core`, instantiated ×4) — abs.w (4 B) in the measurement, abs.l (6 B) at the real base |

Both rounds' measurements were `distorted` (the collision fallback fired: round 0 on
`rings` [0x37F4,0x39BA) vs `entity_window` [0x39AC,...) — the injected growth itself;
round 1 on the innocent pair), and the walk exited on "lengths stable + still
colliding", which it reads as an anchor overrun.

**Mechanism — aeon's story refuted in detail, the symptom confirmed.** The tables live
at 0x296F0..0x2A7F0 (`collision_data`), above $8000 at ANY real base, so an unsized
`lea Table, a1` is abs.l at the provisional pin too; and an UNRESOLVED `RelaxAbsSym`
target is a hard error (`relax.rs` `unresolved_abs_target_diag`), never a short
encoding. What actually happens: when the pinned measuring resolve collides,
`image_lens_pinned(.., scratch_data = true)` parks every pure-data section at a scratch
slot `0x70_0000 + k·0x10_0000`. `collision_data` is slot k = 41 → **0x300_0000**, and
`asl_width_rule` masks the address to 24 bits → **0x0**; `SolidityTable` (offset 0x2100)
therefore looks like `$2100`, abs.w-reachable, and all 12 sites measure 4 B. (The
native.rs comment even claimed the opposite direction — "code measures LONGER while the
data is at scratch".) The spread fallback inherits the same scratch pass. So the
measurement lied SHORT, the pack round placed `section` 24 B into `player_sensors`, and
the "stable-but-colliding" exit mis-labelled it an anchor overrun.

## 2. Design — (A), measurement that cannot lie

Measure every round AT ITS OWN BASES, exactly: labeled sections pinned at the round's
bases (provisional in round 0, packed after), and let the measuring resolve TOLERATE
overlap — a section's length at a pin depends only on label addresses (pin + offset),
never on whether a neighbour's extent intersects. Then measure → pack → re-measure to a
fixed point (≤ 8 rounds, unchanged budget); at the fixed point ONE checked resolve
proves the image sound, and a collision THERE is the real anchor-overrun message. The
scratch/spread fallbacks (the only places a substitute base could steer a width) are
deleted; the residual scratch for label-less blobs / phase banks stays inside the
abs.l window `[0x80_0000, 0xFF_8000)` of the 24-bit space with a loud refusal on
exhaustion. (C) exists as the non-convergence fallback: the diagnostic names each
section whose length still moves and the `file:line` of every width-flipping site with
both encodings' lengths.

**Why part of the fix lands in `sigil-link` (the brief named `native.rs`):**
`resolve_layout` refuses an overlapping layout after its fixpoint (check c2) and that
refusal is exactly what forced `native.rs` to measure at substitute bases. The
measuring device needs the relaxer's exact widths at colliding pins, so the seam is a
`resolve_layout_measuring` variant in `sigil-link/src/relax.rs` that runs the same
fixpoint and skips only the image-soundness checks (overlap, bank straddle) —
`resolve_layout` itself is unchanged (a wrapper over the shared body). All planning
logic stays in `native.rs`.

(B) (pack pure data first) was rejected: it still measures code at a base that is not
its final one, so it only narrows the window; (A) closes it.

## 3–5. Identity, gates, suite, timing — filled as steps land
