#!/usr/bin/env bash
# Probe runner: assemble one file with the committed reference asl and print
# the listing plus the p2bin image bytes. Not part of the suite.
#
# THE ASSEMBLER IS SELECTED BY DIGEST, NOT BY BANNER. This corpus is the one
# where that matters most: `wrange.asm` and `wimm.asm` deliberately write
# operands asl refuses, and the build this runner used to name substitutes an
# UNINITIALIZED WORD for a refused operand — `303C 5602`, `303C 55B1`,
# `303C 5655`, `303C 557F` on four consecutive runs of `wrange.asm`, against
# `303C 8000` every time from the reference build. See `../asl-reference/`.
#
# THAT `303C 8000` IS NOT AN ANSWER EITHER, and this comment used to read as
# though it were. It is `wrange.asm`'s line 5 — `move.w #-32768,d0`, in range,
# ACCEPTED, legitimately $8000 — leaking into the four refused lines below it,
# which echo the last value asl computed. `wcarry.asm` beside `wrange.asm` is
# the control: same file, line 5's accepted value changed to $1234, refused
# lines untouched, and all four then read `303C 1234`. `wcarry0.asm` has nothing
# accepted above its refused lines and reads `0000`.
#
# So the digest picks WHICH build answered; it cannot make a refused line have
# an answer. Do not pin the byte column of a refused line from either build.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
f="$1"
base="${f%.asm}"
cd "$HERE"
rm -f "$base.p" "$base.lst" "$base.bin"
AS_MSGPATH="$ASLDIR" "$ASLDIR/asl" -L -q -i "$HERE" "$f"
echo "ASL_EXIT=$?"
echo "=== LISTING ==="
cat "$base.lst" 2>/dev/null
echo "=== P2BIN ==="
AS_MSGPATH="$ASLDIR" "$ASLDIR/p2bin" "$base.p" "$base.bin" 2>&1
echo "P2BIN_EXIT=$?"
echo "=== IMAGE ==="
xxd "$base.bin" 2>/dev/null
