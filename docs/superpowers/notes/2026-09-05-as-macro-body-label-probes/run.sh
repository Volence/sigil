#!/usr/bin/env bash
# Probe runner: assemble one file with the reference asl and print the listing.
# Flags are Sonic 1's own (build_tools/lua/common.lua:773) minus `-E` (which
# would redirect diagnostics to a file) and minus `-c`. Not part of the suite.
#
# The assembler is selected by MD5, not by path and not by version banner: four
# `asl` binaries in this workspace print the same banner and are not the same
# program, and one of them answers refused operands from uninitialized memory.
# `asl_ref.sh` refuses anything but the reference build; `|| exit $?` is
# load-bearing because `set -uo pipefail` is not `set -e`.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
f="$1"
base="${f%.asm}"
cd "$HERE"
rm -f "$base.p" "$base.lst"
AS_MSGPATH="$ASLDIR" "$ASLDIR/asl" -xx -n -q -A -L -U -i "$HERE" "$f"
echo "ASL_EXIT=$?"
echo "=== LISTING ==="
cat "$base.lst" 2>/dev/null
