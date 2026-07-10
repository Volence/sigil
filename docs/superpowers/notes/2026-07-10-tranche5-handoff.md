# Tranche 5 handoff — game_loop + sound_api (written 2026-07-10, post tranche-4 close)

For the next session. Tranche 4 is FULLY CLOSED AND PUSHED both sides
(sigil master `cc4535a`, aeon master `6fe1388`): D2.33 build + three ports
(particle_anims / sonic_anims / act_descriptor) + the step-5 pad-drop/
inline rewrite + all three rulings (I1 scalar-le policed, M6 split, D2.34
reverse-seam ordinal proof stage 1). Strict **1944/0**, clippy clean.
Packet: `notes/2026-07-10-tranche4-packet.md`.

## Tranche 5 scope (Volence-ratified 2026-07-10)

**A (the spine): `engine/system/game_loop.asm` + `engine/sound/sound_api.asm`.**
**B (stretch, if A lands fast): the first real OBJECT port**
(`games/sonic4/objects/test_solid.asm` or `test_particle.asm` — opens the
object-bank neighborhood: SST custom overlays, spawn templates,
objroutine dispatch).

Branches ALREADY OPEN: sigil worktree `.worktrees/port-tranche5` (branch
`port-tranche5` off `cc4535a`), aeon branch `sigil-emp-tranche5` in the
MAIN tree (off `6fe1388`). Aeon is currently checked out on that branch.

## The two hazard classes A exists to settle (design DELIBERATELY, step 0)

game_loop.asm is 22 lines but carries both:

1. **`ifdef SOUND_DRIVER_ENABLED` inside a ported file** — the .emp needs
   build-shape conditionals. Machinery half-exists: mt_bank.emp takes a
   `DEBUG` define (comptime `if` over defines). The gate builds must pass
   the define through `placed_module_sections` like mt_bank does — BUT
   note the gates' shapes: SOUND_DRIVER_ENABLED is ON in every reference
   build (plain AND debug), so the ported file is shape-invariant in
   practice; the conditional still needs spelling for the demo/no-sound
   build (`games/demo` boots WITHOUT sound — check how demo builds gate
   this; the ported .emp must serve BOTH games or the gate define must be
   sonic4-only like all prior ones).
2. **`gameDebugTick` — a GAME-defined macro the ENGINE invokes** (the
   game-contract macro seam; see main.asm's MACRO LAW block and
   engine.inc's contract comment). `.emp` has no macro-invocation-of-an-
   AS-macro. Design options to weigh at step 0:
   (a) keep the macro AS-side: the gate splits game_loop so the
       `gameDebugTick` line stays in the AS twin only — WRONG (gate-on
       shape would lose the hook);
   (b) the .emp spells it as a cross-seam CALL (`jbsr` to a label the
       game contract provides instead of a macro) — changes the AS
       contract (macro → proc), byte-DIFFERENT if the macro body was
       inlined (sonic4's gameDebugTick body: check
       games/sonic4/config/game.asm — if it's a bsr already, proc-ifying
       is byte-neutral-ish);
   (c) an .emp `extern-macro` construct (new language surface — only if
       (b) is genuinely worse).
   Recon FIRST: read sonic4's gameDebugTick macro body + the demo game's;
   measure what the inlined bytes actually are in the reference. The
   decision is exactly the kind the campaign exists to make deliberately.

Also in game_loop: `movea.l (Game_State).w, a0 / jsr (a0)` — computed
jsr (bare jsr is CORRECT per the jbra/jbsr rule: computed target) and
`bra.s` self-loop.

## sound_api.asm notes

263 lines, engine/sound/, under `SOUND_DRIVER_ENABLED` at the include
site (engine.inc). Real code; the R3 imm32 deferral's original home
(`movea.l #SongTable` etc. — those symbols are .emp-side under
SIGIL_EMP_MT, so the mixed gate already defers them — porting sound_api
FLIPS those to .emp-side referencing .emp-side). Region bounds from the
listings as usual.

## Standing discipline (unchanged — see tranche-4 packet for the loop)

Worktree for sigil, aeon branch in MAIN tree, TDD, per-item two-stage
reviews + two-prong whole-branch review, byte gates both shapes, negative
probes, gate-off neutrality sha256 ×3, kill-list rows land IN THE SAME
COMMIT as any new mirror, Volence checkpoint before merge, step-5 queue
post-merge. **cd EXPLICITLY on every command** (the cwd reset bit ~5
times last session). Current reference pins (PROVENANCE tail): non-debug
`907a9029…`, debug `7148f938…`.

## Carry-forward / open

- Kill-list row 4 stage 2 + the `.b`/`.w` imm-deferral extension (both
  Spec-5-era; ledgered).
- Volence has STRUCTURAL engine changes queued (his note at the tranche-4
  close) — out of campaign scope, but expect reference re-pins when they
  land; re-baseline per PROVENANCE.
- Stretch-B object port wants the SST overlay + spawn-template constructs
  exercised — read `examples/sst_overlay.emp` + the pitcher_plant
  exhibits before starting.
- Empyrean amendment stack (D2.33 + 2026-07-10b + D2.34) still
  UNCOMMITTED in the working tree (Volence's docs cadence).
