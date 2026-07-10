# The 6/7 gap — construct-walk #3, then tranche 7 (written 2026-07-10, post tranche-6 merge)

Tranche 6 is MERGED AND PUSHED both sides (sigil master `91a3f51`, aeon
master `2c0a1f4`; strict **1998/0**; pins plain `588adf81…` / debug
`ed96301f…`; packet: `notes/2026-07-10-tranche6-packet.md`).

## FIRST: construct-walk #3 (Volence-ratified into this gap)

**The Sonic newtype set vs player physics — ~30 min, VOLENCE DRIVING.**
Ordered BEFORE tranche 7 so collision.asm ports ONCE into the final typed
surface. Materials:
- the candidates memory (`emp-sonic-newtype-candidates`): Angle,
  SubPixel/Speed fixed pair, VramTile + conversion, Tile/Block/Chunk,
  palette/collision/sound ids;
- the target surface: aeon `engine/objects/sst.emp` (authored walk-ready —
  raw ints, fields grouped so newtypes land as annotation diffs);
- the demand evidence: `engine/objects/collision.asm` (32 SST refs) +
  `games/sonic4/player/player_ground.asm`-class hot code — read the
  candidates AGAINST these, don't design in a vacuum;
- typed data-register params (`d0: Angle`) are CONFIRMED WORKING and have
  been waiting for this walk since tranche 3.
Output: the ratified type set applied to sst.emp + back-propped onto the
two object modules (a small annotation diff; the loop's step-4 mechanics).

## THEN: tranche 7 — engine/objects/collision.asm

232 ln, 32 SST refs, hot bug-fix file; its step-5 engine review IS the
point. **GATE (re-ask, was "not sure yet" at tranche-6 kickoff): do
Volence's queued STRUCTURAL engine changes touch collision.asm? If yes,
those land aeon-side first** (transcribe the file he intends to keep). Its
AABB shared macro (`engine/objects/aabb.inc`) is the ledgered
"second consumer" demand for a shared engine-macros templates module.

## Carried context

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
