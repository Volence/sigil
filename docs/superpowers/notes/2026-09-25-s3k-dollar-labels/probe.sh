#!/usr/bin/env bash
# probe.sh <name>: assemble probes/<name>.asm with the pinned asl (asl_run) and with
# this parcel's sigil; print both exits, asl's listing, and both images as hex.
# asl bytes come from p2bin over asl's .p, and ONLY when asl exited 0.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-dollar-labels
. /home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a620ff60a900c195e/docs/superpowers/notes/asl-reference/asl_ref.sh || exit $?
P2BIN=$ASLDIR/p2bin
N="$1"
cd "$S/probes" || exit 9
rm -f "$N.p" "$N.lst" "$N.asl.bin" "$N.sigil.bin" "$N.h"
echo "=== $N ($(md5sum "$N.asm" | cut -c1-8))"
asl_run -xx -n -q -A -L -U -i . "$N.asm" > "$N.asl.out" 2>&1
arc=$?
grep -E 'ASL_EXIT|ASL_DIAG|error|warning' "$N.asl.out" | head -12
if [ $arc -eq 0 ]; then
  "$P2BIN" "$N.p" "$N.asl.bin" -p=0 > /dev/null 2>&1
  echo "asl   bytes: $(xxd -s ${SKIP:-0x1200} -p -c 256 "$N.asl.bin" | tr -d '\n')"
else
  echo "asl   bytes: (asl exited $arc; no value quoted)"
  grep -iE 'error|warn' "$N.lst" 2>/dev/null | head -8
fi
"${SIGIL:-$S/target/release/sigil}" "$N.asm" -o "$N.sigil.bin" > "$N.sigil.out" 2>&1
src=$?
echo "SIGIL_EXIT=$src"
grep -E 'error|warning' "$N.sigil.out" | head -8
if [ -f "$N.sigil.bin" ] && [ $src -eq 0 ]; then
  echo "sigil bytes: $(xxd -s ${SKIP:-0x1200} -p -c 256 "$N.sigil.bin" | tr -d '\n')"
fi
if [ $arc -eq 0 ] && [ $src -eq 0 ]; then
  if cmp -s "$N.asl.bin" "$N.sigil.bin"; then echo "MATCH"; else echo "DIFFER"; fi
fi
