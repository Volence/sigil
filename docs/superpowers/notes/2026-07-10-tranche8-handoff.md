# Tranche 8 handoff — rings.asm (written 2026-07-10; tranches 7 + 7b SHIPPED)

Tranche 7 (collision.asm) AND the 7b follow-up (interact-pointer
staleness fix, Volence-approved in advance) are MERGED AND PUSHED both
sides: sigil master `c1cdd78`, aeon master `b49ea8d`; strict 2034/0;
pins plain `e22a82b3…` / debug `0c9f1952…`. Packets:
`notes/2026-07-10-tranche7-packet.md` + `notes/2026-07-10-tranche7b-interact-fix.md`.

## Tranche 8 — engine/objects/rings.asm (recommended, not yet ratified)

Why rings: (1) it is kill-list row 13's KILL CONDITION — the aabb.inc
macro twin dies when its last AS consumer ports; (2) it has now inherited
TWO changes sight-unseen (the alias-skip peephole through the shared
.inc, and the −8/−36 slides around it); (3) RingCollision is a hot loop
(per-frame over the ring buffer) — step-5 value; (4) it exercises the
aabb.emp template with the OTHER call shapes: `(a0)` and `2(a0)` —
**the zero-disp collapse probe promised in row 13 happens here**
(boff=0 must collapse `0(a0)` → `(a0)` for asl parity).

Alternative if Volence prefers: animate.asm (AnimId/FrameId typed
surface + kill rows 2/3's animate.asm flip gets closer).

## Carried context

- Worktrees were REMOVED at tranche close. Fresh aeon worktrees MUST be
  seeded: `cp -rp games/sonic4/data/editor .worktrees/<wt>/games/sonic4/data/`
  (gitignored Aurora editor .bin data is a build input — without it the
  generators emit air-baseline collision and the ROM silently diverges
  130KB; ledger row asks for a build.sh warning).
- Re-pin discipline (learned twice): a region byte-change re-derives
  EVERY `SIGIL_EMP_*` org between it and the next org boundary, and a
  gameEngineBlockIncludes file (player_sensors etc.) changing makes the
  slide TWO-STAGE. All numbers from listings, never arithmetic.
- Constants-twin growth is now ONE list:
  `sigil-harness/src/test_support.rs::engine_constant_equs()` (18
  entries) — grow twin → grow list → done.
- Open language asks with data points waiting: reglist ranges in
  clobbers (3 data points), local typed reg binding (`let a2: *Sst`),
  branch_table dispatch encoding (the bra.w-stride table), role-typed
  SST views (FIRST demand recorded in 7b — second ratifies), fn-scoped
  template hygiene (Code++ per-fragment spaces), F3 deep case (imported
  fn reading home-module privates).
- Empyrean amendment stack (Volence's cadence) now carries: tranche-5
  list + tranche-6 four + tranche-7's utag-death/F1-F4 surfaces/Code++
  semantics + 7b's SST_interact convention.
- Oracle: scroll-test maps Up/Down to direct player movement — account
  for it in fabrication scenarios. Object fabrication recipe (48-byte
  slot images, mappings borrowed from Player_1) is in this session's
  tranche-7 work if needed again.
- cd EXPLICITLY on every command (bit twice again this session — one
  merge ran in a worktree, one test run in aeon).
