#!/usr/bin/env bash
# Probe runner: assemble one file with the committed reference asl and print the
# listing. Flags are Sonic 1's own (build_tools/lua/common.lua:773) minus `-E`
# (which would redirect diagnostics to a file) and minus `-c`. Not part of the
# suite.
set -uo pipefail
ASLDIR=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64
HERE="$(cd "$(dirname "$0")" && pwd)"
f="$1"
base="${f%.asm}"
cd "$HERE"
rm -f "$base.p" "$base.lst"
AS_MSGPATH="$ASLDIR" "$ASLDIR/asl" -xx -n -q -A -L -U -i "$HERE" "$f"
echo "ASL_EXIT=$?"
echo "=== LISTING ==="
cat "$base.lst" 2>/dev/null
