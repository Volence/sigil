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
#
# EVERY invocation is under `timeout`: this set deliberately asks asl what it
# does with a file that includes itself, and an assembler that answers by
# recursing forever would otherwise hang the run. A timeout here is a
# MEASUREMENT — `ASL_EXIT=124` is the answer "asl did not stop", not a flake to
# retry. TIMEOUT_S is overridable so the same script can distinguish "slow" from
# "unbounded".
set -uo pipefail
TIMEOUT_S="${TIMEOUT_S:-25}"
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
f="$1"
base="${f%.asm}"
cd "$HERE"
rm -f "$base.p" "$base.lst"
AS_MSGPATH="$ASLDIR" timeout "$TIMEOUT_S" "$ASLDIR/asl" -xx -n -q -A -L -U -i "$HERE" "$f"
echo "ASL_EXIT=$?"
echo "=== LISTING ==="
cat "$base.lst" 2>/dev/null
