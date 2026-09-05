#!/bin/sh
# img.sh <base> — assemble probes/<base>.asm with asl and print the emitted
# image as hex, plus the asl exit code and any diagnostics.
set -u
D=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-af765a391dc6a9497/probes
B=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64
cd "$D" || exit 1
rm -f "$1.p" "$1.bin"
DIAG=$(AS_MSGPATH=$B USEANSI=n "$B/asl" -U -q "$1.asm" 2>&1)
RC=$?
echo "--- $1: asl_exit=$RC"
[ -n "$DIAG" ] && echo "$DIAG"
if [ -f "$1.p" ]; then
  "$B/p2bin" "$1.p" "$1.bin" -k >/dev/null 2>&1
  printf 'image: '
  od -An -tx1 -v "$1.bin" | tr -s ' ' | tr -d '\n'
  echo
else
  echo "image: <no .p produced>"
fi
