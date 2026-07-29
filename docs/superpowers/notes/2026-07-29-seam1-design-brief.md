# 2026-07-29 — seam-1 brief: the resident-blob NATIVE-LINK design (DESIGN GATE ONLY)

Status: **DISPATCH BRIEF — DESIGN-ONLY** (overseer: Fable; porter: Opus subagent).
The seam sub-tranche's input set is FINAL (all six sound twins: kill rows 70/71/78/
83/87 + sound_api; the 4 outbound driver externs; the drift-diagnostic ledger set).
This parcel produces the DESIGN for making the six sound `.emp` files link natively
as one blob — **no implementation, no twin deletion, no build change**. The actual
seam execution (which RETIRES the six `.asm` twins) is a separate, Volence-visible
event dispatched only after this design is endorsed. Sigil master = THIS brief's
commit; aeon master **`597ce06`**.

## 0. Bars

- Read-only against aeon except the design note; sigil branch `seam1-design`,
  worktree `.worktrees/seam1-design` (doc parcels mandate worktrees). Canonical
  4b66cace/1c256b3b; strict 2888/0 (1 ignored) — verify before starting.
- Deliverable = ONE committed design note + a STOP. Standard rules (cd-every-call,
  explicit paths, no pushes).

## 1. The design questions (from the accumulated ledger; answer each with evidence)

1. **The link shape**: today the blob is AS-phased ($0000 origin, `phase 0`,
   Z80-blob-precedes-engine) with each `.emp` proven by a windowed oracle against
   its twin's bytes. Native link = sigil assembles the six `.emp` files as ONE Z80
   region feeding the 68k build. Design: the region/manifest form, the origin/phase
   handling (the driver's framing origin + `Z80_Sound_End=$1BFA`), the per-file
   ordering (driver → sequencer → sfx → fm → psg + api — VERIFY the true blob order
   from the listings), and how the 68k side includes the blob (the mixed-link path
   the combined-link fix hardened — cite its classes).
2. **The identity bar**: the first native link must produce the EXACT current blob
   bytes (both shapes — the +$7E internal sequencer growth and the driver's 9
   cross-seam operand bytes must fall out of the real link, not pins). Design the
   proof: whole-blob byte gate vs the current ROM extraction, both shapes, plus the
   downstream-unchanged proof (the 68k corpus after the blob).
3. **The retirement mechanics**: which files/arms/pins/tests die (the six twins, the
   windowed oracles' AS-reassembly halves, the equ carriers, the cross-file extern
   decls incl. psg's 3 + sfx's 13 + the driver's 4) and which SURVIVE transformed
   (the windowed gates become native-blob gates? the drift guards?). Every kill-list
   row touched gets its planned disposition.
4. **The drift diagnostics land HERE**: the extern-decl-vs-def check becomes moot
   for the deleted decls, but the TRANSITIVE clobbers-completeness fixpoint
   (`[call.clobbers-incomplete]`) and any cross-module contract closure become
   BUILDABLE against a single linked module set. Design what ships with the seam vs
   what stays ledgered — the 5-face drift set is the checklist.
5. **Sequencing**: one seam or two (the board says "2 seams" — the resident blob vs
   the banked/data side; scope THIS design to the resident blob and NAME what the
   second seam owns). Where the generator conversion and the data/BINCLUDE files
   sit relative to the flip.

## 2. STOP

Commit the design note, report, STOP. The overseer reviews, then surfaces the
execution plan (with the twin-retirement consequence stated plainly) to Volence
before any deletion is dispatched.
