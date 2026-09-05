#!/bin/sh
# img.sh <base> — assemble <base>.asm with the reference asl and print the
# emitted image as hex, plus the asl exit code and any diagnostics.
#
# THE ASSEMBLER IS SELECTED BY DIGEST, NOT BY BANNER — see `../asl-reference/`.
# This runner prints an IMAGE, which is the shape most exposed to the varying
# build: that one substitutes an uninitialized word for any operand it declined
# to give a value, with exit 0 and no diagnostic, so a wrong image here would
# arrive looking exactly like a right one.
#
# THE PROBES ARE THIS DIRECTORY'S OWN, not an untracked copy in another
# worktree.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
cd "$HERE" || exit 1
rm -f "$1.p" "$1.bin"
DIAG=$(USEANSI=n "$ASL" -U -q "$1.asm" 2>&1)
RC=$?
echo "--- $1: asl_exit=$RC"
[ -n "$DIAG" ] && echo "$DIAG"
if [ -f "$1.p" ]; then
  "$ASLDIR/p2bin" "$1.p" "$1.bin" -k >/dev/null 2>&1
  printf 'image: '
  od -An -tx1 -v "$1.bin" | tr -s ' ' | tr -d '\n'
  echo
else
  echo "image: <no .p produced>"
fi
