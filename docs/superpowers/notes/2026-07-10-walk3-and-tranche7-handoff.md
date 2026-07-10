# Tranche 7 handoff — collision.asm into the typed surface (written 2026-07-10; walk #3 DONE)

Tranche 6 is MERGED AND PUSHED both sides, and **construct-walk #3 RAN AND
SHIPPED same day** (Volence driving; sigil master `c779977`, aeon master
`d3cf26b`; strict **1998/0**; pins plain `588adf81…` / debug `ed96301f…`;
tranche packet: `notes/2026-07-10-tranche6-packet.md`).

## Walk #3 is DONE — the typed surface exists

Rulings + rationale: `notes/2026-07-10-construct-walk3-outcome.md`. The
vocabulary lives in aeon `engine/system/types.emp` (zero-byte module):
**Coord** fixed<16,16> / **Velocity** fixed<8,8> / **Angle** u8 /
ObjRoutine / Radius / VramArtTile / AnimId / FrameId. sst.emp is retyped;
GetSineCosine takes `(d0: Angle)` and returns the BARE fixed<8,8> fraction
(NOT Velocity — ruled, ledgered with the out-typing ask). Modules that
`use` sst.emp or math.emp need `types.emp` items ambient
(`types_ambient_items` in the mixed harness; the port tests show the
pattern).

## Tranche 7 — engine/objects/collision.asm

232 ln, 32 SST refs, hot bug-fix file; its step-5 engine review IS the
point. **GATE RESOLVED at T7 kickoff (2026-07-10): the "queued structural
engine changes" was Volence thinking out loud at the tranche-4 close —
nothing is queued. Question retired permanently; do NOT carry it into
future handoffs.** collision.asm ports as-is from aeon master. Its
AABB shared macro (`engine/objects/aabb.inc`) is the ledgered
"second consumer" demand for a shared engine-macros templates module.

## Carried context

- Collision ports INTO the typed surface: Coord/Velocity/Radius/Angle all
  sit in its hot path — spell field access off `Sst` from day one.
- Headline ledger ask from tranche 6: **label values in imm exprs**
  (every object port self-externs its own Main for the objroutine store
  until it lands). Also open: equ hygiene (link-global non-pub equs),
  clobbers() reglist ranges, use-import of offsets-table labels.
- Empyrean amendment stack (Volence's docs cadence) now carries tranche 5's
  list + tranche 6's four: `.w` ImmLink, emp zero-disp collapse, AS `dc.w`
  Value16Be deferral (+ recorded asl divergence), fixup dangling-leaf naming.
- Kill list: 12 rows (11 = sst.emp struct twin, 12 = RF mirrors; row 2
  consolidated into the constants twin).
- PROCESS: `DEBUG=1 ./build.sh` OVERWRITES `s4.bin` — the debug reference is
  a manual `cp s4.bin s4.debug.bin; cp s4.lst s4.debug.lst` AFTER a
  DEBUG=1 build (a stale-file check got caught this tranche; consider a
  build.sh output-name switch). Oracle: breakpoint-resume re-breaks at the
  same PC — frame-step with `emulator_press` instead (fix jotted).
- **cd EXPLICITLY on every command** (the harness resets cwd — bit again
  this session, including one cargo run that silently executed in aeon).
