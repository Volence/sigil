# 2026-07-29 — t38 step-2 opening + retirement-checklist survey (continuation handoff)

Porter: t38 continuation. Follows the verifier feature commit
(`2026-07-29-t38-sp-cleanup-verifier.md`). This note records what the step-2
OPENING commit shipped, the retirement-checklist survey outcomes, and the
concrete map of what remains (checkpoint (b) is NOT yet reached).

## Committed this session

- sigil `be78538` — the immediate-sp-cleanup verifier feature (9 sp_cleanup
  tests, full frontend suite 1730/0). Its own note documents it.
- sigil `4920014` — kill row 80 CLOSED.
- aeon `6f22d36` — the step-2 opening: 8 68k extern-proc decls deleted +
  preserves(a0) on the 4 cores + the d6 catch + 3 `bsr.s .cell`→`jbsr .cell`.

Full strict suite (AEON_DIR = the aeon worktree): **2851 / 0 / 1** — the baseline
closure-residue failure cleared, +9 sp_cleanup tests. Byte gates green both
shapes: test_p4_player_sensors_port, test_p2_player_states_port,
test_p1_player_port, mixed_tranche35/38.

## The a0-firing root cause had TWO faces (the ruling named ONE)

The overseer ruling addressed the immediate-sp-cleanup idiom only. Implementing it
alone did NOT clear the 10 firings — a SECOND root cause surfaced:

**The CFG classifies `bsr` as a conditional branch** (`edges()`:
`mnem.starts_with('b') && mnem.len() == 3` matches "bsr"), so `bsr.s .cell` flows
INTO the `.cell` local subroutine and treats its `rts` as the enclosing proc's
return — where a0 is legitimately clobbered (Collision_GetType) and not yet
restored (the restore is in the OUTER body). That path refutes preserves(a0).
`jbsr`/`jsr` do NOT match the cond-branch shape → correctly modeled as calls
(Follow-next-only, never flowing into the target). `bsr` and `jbsr` are the SAME
instruction, so this is a latent CFG ASYMMETRY.

RESOLUTION (byte-neutral, in the opening commit): convert the 3 `bsr.s .cell`
local calls to `jbsr .cell` (short reach → jbsr picks bsr.s → byte-identical).
This is house format (step-2 mandates bsr→jbsr) AND fixes the modeling. The AS
twin keeps `bsr.s`.

**LEDGER / FINDING for the overseer:** the `bsr <local>` cond-branch
mis-classification is a latent verifier gap — any future un-modernized `bsr .L` in
a preserves-checked proc re-hits it. The bsr→jbsr house format masks it corpus-
wide (post-step-2 no `bsr` remains in `.emp`), so severity is LOW, but the CFG
`edges()` should model CALL_MNEMONICS (jsr/jbsr/**bsr**) uniformly as calls. NOT
fixed here (it is a shared-CFG change with flag_check blast radius — the jbsr
conversion is the lower-risk, campaign-aligned resolution). This is a FOURTH-face
cousin of the drift diagnostic, verifier-side.

## Retirement-checklist survey (brief §3) — outcomes

1. **Row 80 (8 extern-proc decls)** — **EXECUTED.** All 8 deleted (7 sensor + 1
   AtLedgeEdge); no new 68k extern proc born (Collision_GetType bare-links to the
   ported engine.collision_lookup — recon predicted a new decl, the port needed
   none: **corrections row**). 68k game side = ZERO extern proc (t28 headline for
   68k). Kill row 80 closed.
2. **Player_AtLedgeEdge boundary decl (t34 die-at-port)** — **EXECUTED** (part of
   the 8). PLUS the d6 under-claim catch (see below).
3. **The 5 guarded PlayerV fields (row 74)** — **CONDITION NOT MET** (recon §4,
   re-confirmed). `_pl_*` readers survive in player_{ground,air,spindash,common}.asm
   (the row-79 gate-off twins) + config/game.asm. player_sensors reads NO `_pl_*`.
   Rows 72/74/75 do NOT collapse at P4 — their true kill is Spec 5. **corrections
   row: the brief's "sensors is the LAST `_pl_*` reader" premise is FALSE.**
4. **offsets-construct adoption (Player_States, player_common.emp:500)** — **NOT
   DONE (finding, deferred).** No corpus precedent for an `offsets Name {}`
   declaration (grep: zero adoptions). The 3 tables' terms are CROSS-MODULE
   externs (`extern("PState_Ground")` etc. — PState_* live in player_ground/air/
   spindash modules, PHook_* local). Whether `offsets` accepts cross-module extern
   targets is UNVERIFIED; if not, adoption needs a frontend increment (construct-
   feature scale, like the t38 dc.w blocker), not a byte-neutral tidy. Needs the
   full step-3(a)-ask / step-4-build treatment, not a drive-by swap. The current
   `extern(target) - extern(base)` form already lowers to the same Cell::RelOffset
   words the offsets construct would emit, so there is no correctness gap — only a
   readability adoption.
5. **Row 81 P4 arm** — player_sensors uses ZERO `PPHYS_*` (grep confirmed). Its
   own SOLID_TOP/LRB mirror is a fresh 1st-consumer drift-guarded mirror, not a
   row-81 item. No game-constants `.emp` module born this tranche → row 81 stays
   open (kill = the module is born, or Spec 5). Condition not fired by sensors.

## The d6 under-claim catch (packet item)

Player_AtLedgeEdge's balance-path ledge probe WRITES d6, but the t34 header +
caller decls declared `clobbers(d0-d5/a1-a2)`. Corrected to `d0-d6`, propagated to
its callers Player_Animate / Player_Display (`d0-d5`→`d0-d6`). This is the
campaign's **FIRST 68k-side header UNDER-claim** (Z80 had three, all in sound_psg/
sound_fm). The over/under-claim scoreboard now tracks BOTH CPUs.

## What REMAINS (the continuation's map — checkpoint (b) NOT reached)

- **Step-2 house format proper** (byte-CHANGING wave): the file-wide bra→jbra +
  bsr.w→jbsr relaxations (some bsr.w may relax to bsr.s → region shrink → the FULL
  wave-ripple checklist: repin, twin lockstep, downstream slide, $8000 bar,
  neighbour canaries, per-region delta table), brace-indent, the idiom walk
  (Sst.field / bare-abs-EA / contract clause order), the type-layer walk (the
  sensors carry Angle/Coord-flavored domain values — candidates).
- **offsets adoption** (item 4 above) — as a step-3(a) ask / step-4 build.
- **The 3→4→5 loop until dry**, then the **dry-panel** (A1+B1+C2+C1 active named
  sites; C2 owes the RelOffset emission re-derivation + a probe-core comptime-fn
  re-derivation + the post-deletion contract re-check per the brief) → STOP at
  checkpoint (b) with the full evidence block + retirement-checklist outcomes
  table.
- Packet items to carry: the a0 gap was latent in the countersigned (a) (masked by
  the closure gate's panic order — single-failure-class reporting ledger note);
  the drift diagnostic now has FOUR faces (Z80 silent-tolerate / Z80 local-pass /
  68k refuse-outright / 68k verifier-precision-limit) + this note's fifth cousin
  (the bsr-vs-jbsr CFG asymmetry, verifier-side).
