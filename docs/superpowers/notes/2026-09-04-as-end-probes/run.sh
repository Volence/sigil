#!/usr/bin/env bash
# Probe runner: assemble one file with the committed reference asl and print
# the listing plus the p2bin image bytes. Not part of the suite.
set -uo pipefail
ASLDIR=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64
HERE="$(cd "$(dirname "$0")" && pwd)"
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
