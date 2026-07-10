# Tranche 4 checkpoint packet — COMPLETE (2026-07-10)

Third tranche under the 5-step loop. Volence ratified the close verbally
("looks good — close the loop"); this packet records what shipped, the
retrospect, and the asks that stay OPEN for later rulings.

## What shipped

- **Opening build (D2.33)**: comptime indexing (`base[i]`, postfix `.field`,
  total bounds) + element-typed Data views (verbatim byte-identity, element
  policing). Adversarial review run; its Critical (huge-index usize wrap)
  fixed and pinned. Spec amended (header + §2 row + §4.5/§6.7/§6.8 —
  empyrean working tree, Volence's docs cadence).
- **Port #1 `particle_anims.emp`** — first in-tree `offsets` consumer
  (inline body); first GAME-DATA gate (main.asm, past `org $10000`).
- **Port #2 `sonic_anims.emp`** — the ordinals story: declaration position
  IS the ANIM_* id, 15 drift guards, member-reorder probe.
- **Port #3 `act_descriptor.emp`** — the Tier-1+2 act shape (typed Act/Sec
  literals, validating constructor, comptime invariants, extern value
  cells + residual trees). Byte-exact on the FIRST full compile.
- **Demanded features, shipped mid-port**: imm32 deferral for `d16(An)`
  destinations (port #1); pinned-width `lea (Sym).w/.l, aN` deferral (port
  #3 — the tranche-3 re-scoped "abs-sym mode", now real).
- **Eleven-module mixed gates** byte-identical both shapes; strict
  workspace **1941/0**; clippy clean; neutrality sha256 ×3 at `755c2c91…`.
- **Twin-scaffolding kill list** created (Volence's ask): 8 rows, each with
  a kill condition; same-commit rule for new mirrors.
- Recon corrections: `vram_bases` DROPPED (not in the build);
  `ojz_act_pool` → `act_descriptor` (Volence's catch) — RATIFIED with this
  close.

## Retrospect (step 3) — consolidated

| Finding | Recommendation |
|---|---|
| **Data ports are BORN modern** — transcribe/modernize collapse for offsets/struct shapes | Amend the loop text: steps 1+2 merge for data files; the split stays meaningful for code files |
| **Anims inter-body align pads are dead weight** (engine byte-reads scripts; verified) | STEP-5 QUEUE: drop 5 pads in sonic_anims AND rewrite fully-inline (the pad removal un-blocks the inline form) — −5 bytes + the prettiest source; keep the trailing align (next table's evenness) |
| **The reverse seam falls out of the ordinals story** — export `Ani_Sonic` ordinals AS the `ANIM_*` equs; config block deleted; guards become definitions | OPEN ASK: ratify as the reverse-seam carrier (kill-list row 4's demolition). Needs offsets-ordinal export as link equs (Item-B extension) |
| **`extern()` can't range-prove into u8 fields** (EDGE_CLAMP stayed a mirror) | Working as designed (totality); jot only |
| **comptime fn params: multi-line lists don't parse; `Label` class-checks; loose `int` binds LinkExpr** | Multi-line param lists = small parser polish (jot); the rest is fine |
| **Flash duration folding / Roll comptime-FP interleave** | REJECTED (timing risk / readability) — recorded so nobody re-derives them |
| D2.33 review leftovers: I1 (non-array `le` bypass), M6 (operand-position indexing `move.w Tbl[2],d0` now parses), M2 (carve-out steering), M1 (steer recovery), M4 (splice pin) | OPEN ASKS (rulings), unchanged |
| Tier 3 (mapped section grids) needs computed-name `extern()`; Tier 4 (import()-driven acts) resolves the ojz_act_pool generator question | OPEN — decide at the act-2 moment, not before |

## Open asks (no rush — each is a recorded decision when taken)

1. ANIM-ordinal reverse-seam flip (above).
2. D2.33 review rulings I1/M6 (+M1/M2/M4 polish pool).
3. Step-5 queue for this tranche (post-merge): the anims pad-drop + inline
   rewrite.
4. `ojz_act_pool` generator direction (= Tier 4) — parked until act 2.
5. AF_*/engine-limit constants-twin consolidation triggers (kill-list rows
   2/3/8).

## Post-merge state

Merged --no-ff to sigil master + aeon master pushed (this close); the
worktree removed, branch deleted. The empyrean D2.33 spec amendment stays
in the working tree per the docs cadence.
