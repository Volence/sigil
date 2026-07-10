# Overnight status — tranche 4 (for Volence's morning, 2026-07-10)

Everything below is LOCAL (nothing pushed). Branch `port-tranche4`
(worktree), aeon master has 2 new local commits.

## Done tonight

1. **Port #1 — `particle_anims.emp`** (aeon `b66cb4e` + sigil `694c997`):
   the first in-tree `offsets` consumer. Table word + inline body; the hand
   `$7FFF` guard subsumed; `align 2` at item position; AF_DELETE local
   mirror + extern drift guard. Gate in main.asm (first GAME-DATA gate,
   past `org $10000`). **Feature ride-along the port demanded:** imm32
   deferral extended to `d16(An)` destinations (`move.l #Ani_Particle,
   SST_anim_table(a0)` — the deferred #Sym-immediate item, now with its
   real consumer; encoding-pinned unit test).
2. **Port #2 — `sonic_anims.emp`** (aeon `46cd861` + sigil `c0f06cb`): the
   ordinals story — eleven by-reference members, declaration position IS
   the ANIM_* id, twelve ordinal/count drift guards + three command-byte
   guards, real `align 2` pads between odd bodies, 0x74 bytes
   byte-identical. Probe: a member REORDER trips the ordinal guards.
3. **The TEN-module mixed gates** (`mixed_tranche4_*`): full-ROM
   byte-identical both shapes with both anims regions riding the .emp side,
   all guards checked against the real AS tree. Strict workspace
   **1936/0**, clippy clean. Gate-off neutrality sha256 ×3 at `755c2c91…`.
4. **The act_descriptor design note** (your bedtime ask):
   `notes/2026-07-10-act-descriptor-design.md` — Tier 1+2 (typed Act/Sec
   literals + a shared validating `act()` constructor) recommended for the
   port; Tier 3 (mapped 9-section grids, one small `extern()` increment);
   Tier 4 (acts via `import()` — same decision as the ojz_act_pool
   generator question).

## Waiting on you (the checkpoint packet asks)

- Ratify the recon target swaps: `vram_bases` DROPPED (not in the build),
  `ojz_act_pool` → **`act_descriptor.asm`** as port #3 (your catch), at the
  design note's Tier 1+2 shape.
- The D2.33 review's two open rulings (non-array `le` policing; operand-
  position indexing `move.w Tbl[2], d0` — bless or fence).
- The reverse-seam proof needs a replacement carrier (vram_bases was it).
  Candidate: act_descriptor's port already imports generated equs
  (`OJZ_ACT_POOL_PAGES`) — but the REVERSE direction (.emp equ → AS reads)
  still wants a picked file.

## Earlier today (already seen, recapped)

Step-5 queue done+pushed; D2.33 opening build + review fixes; the three
gameplay fixes (hitbox = stale oracle offsets; balance-on-solids incl. the
object-edge teeter; spindash charge = standing size per classic); oracle
SST decode repair.
