# Tranche 9 handoff — animate.asm (written 2026-07-10; tranche 8 SHIPPED)

Tranche 8 (rings.asm) is MERGED AND PUSHED both sides: sigil master
`7a0f305`, aeon master `07e465c`; strict **2048/0** against master; pins
plain `c973091d…` / debug `6a0f9c3f…` (master ROMs rebuilt post-merge,
listings current). Packet: `notes/2026-07-10-tranche8-packet.md`. Worktrees
removed at close.

## Tranche 9 — engine/objects/animate.asm (RATIFIED — Volence delegated the
pick at the tranche-8 gate)

408 lines: the animation INTERPRETER (AnimateSprite + the per-frame-duration
variant + the control-code/event dispatcher, $F7-$FF). Why animate:
(1) it is kill rows 2/3's named condition (the AF_* block — see hazard 1
before believing the written kill); (2) the AnimId/FrameId typed surface
continues the construct-walk #3 thread ([[emp-sonic-newtype-candidates]] —
anim ids, frame ids as newtypes over the typed-SST fields already in place);
(3) it's HOT engine code (runs per object per frame — real step-5 value,
the interpreter loop + event chain); (4) both its data consumers
(sonic_anims.emp, particle_anims.emp) already ported, so the script FORMAT
is proven .emp-side — this port closes the interpreter half.

Alternative if the session finds animate blocked: entity_window.asm is the
ring system's other half AND the likely ratifier of the packed-record-view
ask (ledger, demand 2/2) — but at 1523 lines it wants its own multi-file
tranche plan, possibly split.

## Step-0 hazards (settle in the design note BEFORE code)

1. **Rows 2/3's kill condition is suspect — apply the row-13 lesson.** The
   written kill ("animate.asm ports → flip") assumed AS readers vanish. They
   don't: `player_common.asm` and `pitcher_plant/anims.asm` (+ the gate-off
   twins sonic_anims/particle_anims/test_particle) read AF_* AS-side, and
   `engine/constants.asm` owns DUR_DYNAMIC. A full ownership flip needs the
   reverse seam (row-4 stage-2 class, Spec-5 era). Likely honest outcome:
   consolidation (mirrors re-homed, AS defs stay until player/pitcher files
   port) — decide and DOCUMENT at step 0; re-verify the written condition
   against the gate-off shape's needs (the tranche-8 lesson, packeted).
2. **`reloadAnimTimer macro srcReg, tag`** — a local AS macro with a
   unique-suffix `tag` param: the utag-death pattern AGAIN (hygiene makes the
   param obsolete). Port as a comptime fn (aabb precedent) or inline it —
   step-0 call. If comptime fn: it stays module-local (no cross-module twin),
   so no new kill row — but the .asm twin keeps the macro (lockstep).
3. **Two `ifdef SOUND_DRIVER_ENABLED` blocks** (lines ~193, ~359 — the
   AF_SOUND event path): the `-D SOUND_DRIVER_ENABLED=0|1` pattern
   (rings/game_loop precedent). Reference gates run SND=1; add the SND combo
   probe vs a fresh AS-twin oracle (rings_port's `as_twin_bytes` shape). No
   `__DEBUG__` code this time (animate has none — simpler than rings).
4. **Region position = the BIGGEST re-pin exposure yet.** animate.asm sits
   UPSTREAM of every gated engine region (engine.inc order: … sprites →
   animate → collision gate $308A/$3344 → rings gate $31F0/$34AA → … →
   $10000). Step 1 is byte-identical (no slide). Any step-2/5 BYTE CHANGE
   slides collision + rings REGION BASES (not just resume orgs) + collision_
   lookup + sound_api + every label pin down to the org-$10000 boundary.
   The GENERALIZED RE-PIN RULE applies (ledger, tranche-8): re-derive EVERY
   harness pin in the sliding window — orgs, map bases (mixed_dac_rom map
   fns!), label VMAs, byte-pin arrays carrying displacements, probe
   constants — all FROM LISTINGS; sweep grep hex literals in the window,
   then let the strict suite name survivors.
5. **AF_CALLBACK dispatch**: two-byte big-endian objroutine offset read from
   an UNALIGNED script stream, then computed transfer — `jmp`/`jsr` reserved
   role (computed targets), plus byte-wise address assembly. Check the
   listing for the exact shape; may exercise new operand forms.
6. **The interpreter is a byte-command reader** — do NOT confuse the port
   with the 9d byte-command DSL (re-gated TWICE, D2.26/D2.27): scripts stay
   data (already ported); this tranche ports the READER as plain procs.
7. **Typed surface (construct-walk #3 thread)**: SST anim fields are typed
   already (sst.emp: anim/prev_anim/anim_frame/anim_timer/anim_table);
   step-2/3 may want AnimId/FrameId newtypes — record as asks unless the
   file DEMANDS them (demanded-features law).

## Mechanics checklist (rings_port.rs is the current best model)

- Branches: sigil `port-tranche9`, aeon worktree `sigil-emp-tranche9` —
  **SEED EDITOR DATA**: `cp -rp games/sonic4/data/editor
  .worktrees/<wt>/games/sonic4/data/` (gitignored build input; without it
  the ROM silently diverges 130KB). Verify baseline hashes == pins BEFORE
  any edit (tranche-8 did; it caught nothing but proves the seed).
- Gate: `ifndef SIGIL_EMP_ANIMATE` in engine.inc + per-shape resume orgs
  from listings; region bounds = animate's first label .. collision's gate
  org ($308A/$3344 — the region END is the collision gate's base).
- Harness: `animate_port.rs` — per-shape Shape (len likely shape-INVARIANT,
  no __DEBUG__ code), drift-guard count = 30 SST + 24 twin + any new
  module-local mirrors; SND combo probe; outbound consumer (AnimateSprite's
  callers: core.asm's object tick); negative probes file; mixed ladder
  `SIGIL_EMP_ANIMATE` + tranche9 map + acceptance both shapes.
- Twin growth: `test_support.rs::engine_constant_equs()` is ONE list and
  every count DERIVES from it (`twin_guards()`) — grow twin → grow list →
  done, counts self-adjust.
- **House format (step 2)**: jbra/jbsr; **BARE conditional branches** (no
  `.s`/`.w` — Volence-ratified 2026-07-10; exceptions only for
  macro-expansion transliterations / twin-locked templates / load-bearing
  tables, commented in place); bare-symbol width-rule spellings; Sst.field;
  sizeof(Sst).
- **Packet format (Volence-ratified 2026-07-10)**: end with "What each pass
  added" — step-3 findings vs step-5 findings PER LOOP PASS, plus the
  neither-bucket headlines. Canonical text in `notes/campaign-port-loop.md`.
- Step-5 live verification in oracle: animation playback is VISUAL — the
  test level's placeholder art animates the ring placeholder + test objects;
  read Sst anim/anim_frame/anim_timer fields per tick, and exercise an
  AF_CALLBACK/AF_SOUND path if a test object uses one. Oracle notes: test
  level spawns 7 rings at y=$60 (x=$80+); scroll-test maps Up/Down to direct
  player movement; teleport via Player_1+2/+6 int words.

## Carried context

- Pins: plain `c973091d14c5cb56…` / debug `6a0f9c3f44986916…` (PROVENANCE.md
  tail has full hashes + the tranche-8 re-baseline notes).
- Kill list: rows 16 (assert transliteration), 17 (sprites geometry
  mirrors), 18 (game-owned ring mirrors) opened at t8; row 13 closed by
  consolidation. Row 1's ensure surface now 24.
- Open language asks with demand counts: `.emp assert`/diagnostics construct
  (1/2 — rings' transliteration); non-SST packed-record view (1/2 —
  entity_window likely ratifies); reglist ranges in clobbers (3 data
  points); local typed reg binding (`let a2: *Sst`); branch_table dispatch
  encoding; role-typed SST views (1/2); fn-scoped template hygiene; F3 deep
  case; `dc` link-expr cells (consumer-gated); Z80 `dc` probe (first Z80
  code port).
- Empyrean amendment stack (Volence's cadence, uncommitted): tranche-5/6/7
  items + 7b SST_interact + t8's dc surface, local-label displacement
  operands, `pea *(pc)` port-translation rule, row-13 lesson, bare-Bcc
  canonicalization.
- Process: cd EXPLICITLY on every command; numbers from LISTINGS never
  arithmetic; merge only after a DRY retrospect + Volence's gate.
