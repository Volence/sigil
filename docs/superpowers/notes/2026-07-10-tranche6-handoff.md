# Tranche 6 handoff — the object-bank opener (written 2026-07-10, post tranche-5 merge)

For the next session. Tranche 5 is MERGED AND PUSHED both sides (sigil
master `e9ca63b`, aeon master `48c1c10`; strict **1985/0**; pins plain
`bcd4e3a5…` / debug `634fea68…`; packet: `notes/2026-07-10-tranche5-packet.md`).

**THE LOOP CHANGED mid-tranche-5 — read `notes/campaign-port-loop.md`
FIRST** (Volence-ratified): transcribe (byte gate lives here only) →
modernize (complete format, bytes may change, lockstep+re-pin) →
retrospect (NAMED deliverable: language/format asks) → back-propagate →
engine-optimize (live-verified) → LOOP UNTIL DRY → merge. No step-5
queue rides behind a merge anymore.

## Tranche 6 scope (Volence-ratified 2026-07-10)

**`games/sonic4/objects/test_solid.asm` (22 ln) +
`games/sonic4/objects/test_particle.asm` (48 ln)** — the object-bank
opener. Both proc-shaped (NOT scripts → `code_word`/S2-D12b stays
parked for the first scripted badnik).

What this tranche opens (all firsts):
- The OBJECT CODE BANK gate class: these live past `org $10000`
  (`ObjCodeBase`, engine.inc ~line 216) via `gameObjectBankIncludes` —
  the gate site is game-side, the org discipline is the bank's.
- `objroutine()` (`x − ObjCodeBase`) — the table-less dispatch fact.
- REAL SST field access + overlays + spawn patterns — the constructs
  from `examples/sst_overlay.emp` + the pitcher_plant exhibits meet
  real code for the first time.
- test_particle's `move.l #Ani_Particle, SST_anim_table(a0)` is the
  tranche-4 imm32-d16 deferral's original consumer — porting it makes
  that seam .emp↔.emp.

## Step 0 (REQUIRED, and it is construct-walk #1's trigger)

Read `examples/sst_overlay.emp`, the pitcher_plant exhibits, the REAL
SST layout (`engine/structs.asm`, `engine/objects/*`,
`engine/macros.asm`), and both target files. This is the kickoff's
**construct-walk #1** — "the production prelude vs the real engine",
flagged highest-drift-risk of the campaign (the mock-prelude class that
hid the table-less dispatcher). The kickoff recommends ~30 min WITH
VOLENCE DRIVING — offer it before soloing the design note.

## Queued behind (tranche 7, decided)

**`engine/objects/collision.asm`** (232 ln, 32 SST refs, hot bug-fix
file) — Volence's ask, ordered AFTER the opener so the SST machinery is
proven before the hard file; its step-5 engine review is the point.
**GATE: ask Volence whether his queued STRUCTURAL engine changes touch
collision.asm — if yes, those land aeon-side first** (transcribe the
file he intends to keep). Its AABB shared macro is the ledgered
"second consumer" demand for lifting stop_z80-class comptime-fn
templates into a shared engine-macros module.

## Standing state

- Kill list: 10 rows (9 = gameDebugTick mirror, 10 = sound_api imm
  mirrors). Gap ledger: preserves(sr) slice SHIPPED; open rows incl.
  pc-rel16 range check, clobbers()-entry validation, word-imm
  truncation parity, `.b`/`.w` imm-link widths (blocks kill-row-4
  stage 2).
- Empyrean amendment stack STILL UNCOMMITTED (Volence's docs cadence):
  D2.33 + 2026-07-10b + D2.34 + the tranche-5 addendum list (packet
  ask 1: comptime-if, ImmLink + here()-in-.l-imm, positional fence,
  sr/ccr operands, preserves(sr)/clobbers(sr), the ratified loop).
- `vram_bases` + `plantbadmaps_anims`: OUT of the build, dead rows.
- **cd EXPLICITLY on every command** (the harness resets cwd after any
  cd — bit twice this session before the habit stuck).
