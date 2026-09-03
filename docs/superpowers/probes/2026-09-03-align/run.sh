#!/bin/sh
# Run one align probe through a chosen asl and print its listing.
#   run.sh <probe-basename> [asl-dir]
# Default asl-dir is the s2disasm flamewing build — the binary the sigil
# corpus is compared against. Pass a different dir to cross-check a build.
set -e
PROBE="$1"
ASLDIR="${2:-/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64}"
export AS_MSGPATH="$ASLDIR"
rm -f "$PROBE.p" "$PROBE.lst"
"$ASLDIR/asl" -xx -n -q -A -L -U -i . "$PROBE.asm" 2>&1 || true
echo "----- listing -----"
sed -n '1,200p' "$PROBE.lst" 2>/dev/null
