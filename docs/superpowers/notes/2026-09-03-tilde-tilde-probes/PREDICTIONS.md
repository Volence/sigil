# Predictions — written 2026-09-03 BEFORE any corpus/aeon/suite run

Author: tilde parcel. Basis: source reads only, no measurement of the outcome yet.

## P1 — Sonic 1 corpus: ZERO movement
`grep -c '~~'` over s1disasm is zero (re-verified below at measure time).
Therefore the S1 total must be UNCHANGED and the per-class SETS must be
identical in BOTH directions (no class gained, none lost, no count moved).
If S1 moves at all, that is a coupling finding, not a number to reconcile.

## P2 — aeon: BYTE-IDENTICAL, all four shapes
Zero `~~` in aeon's `.asm` sources. Expected CRC32/size:
  s4        14ee2440 / 719700
  s4.debug  142294b3 / 737683
  demo      0c456778 / 96474
  demo.debug 2e603d53 / 101339

## P3 — Sonic 2 corpus: MOVES, and I expect a NET RISE
Not zero, and the direction is predicted UP, for a stated mechanism:
every corpus flag behind an `if ~~FLAG` is 0 --
  s2.asm:27  fixBugs = 0
  s2.asm:40  removeJmpTos = 0|(gameRevision>=2)|allOptimizations = 0|0|0 = 0
  s2.asm:49  useFullWaterTables = 0
  s2.asm:68  FixMusicAndSFXDataBugs = fixBugs = 0
  s2.sounddriver.asm:8  FixDriverBugs = fixBugs = 0
  s2.sounddriver.asm:9  OptimiseDriver = 0
Correct `~~0` = 1, so after the fix ~94 `if` bodies that sigil currently SKIPS
become ASSEMBLED for the first time. Newly-visible code can only add
diagnostics; `else` arms that stop being assembled subtract some. Net: rise.
Sets will move in BOTH directions and every rise must be accounted for.

## P4 — the instruction-selection site
`s2.macrosetup.asm:245`
  last_btst_converted := ~~chkop(..) || ~~chkop(..) || ~~chkop(..)
chkop(op,ref) is 1 when op does NOT start with ref. So `~~chkop` is 1 when it
DOES. Correct: tst.b only for the three known bit refs. sigil today: the
predicate is INVERTED, so `tst.b` is emitted for the operands that should get
`btst` and vice versa. Prediction: a byte sweep of `_btst` against asl shows a
tst.b/btst swap before the fix and agreement after.

## P5 — the operator itself
`~~` is a single greedy token. `~~x` = 1 if x==0 else 0. Binds at the unary tier
(tighter than every binary operator). Both asl builds agree.
