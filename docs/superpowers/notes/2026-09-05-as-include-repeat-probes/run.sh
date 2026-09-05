#!/usr/bin/env bash
# Probe runner: assemble one file with the committed reference asl and print the
# listing. Flags are Sonic 1's own (build_tools/lua/common.lua:773) minus `-E`
# (which would redirect diagnostics to a file) and minus `-c`. Not part of the
# suite.
#
# EVERY invocation is under `timeout`: this set deliberately asks asl what it
# does with a file that includes itself, and an assembler that answers by
# recursing forever would otherwise hang the run. A timeout here is a
# MEASUREMENT — `ASL_EXIT=124` is the answer "asl did not stop", not a flake to
# retry. TIMEOUT_S is overridable so the same script can distinguish "slow" from
# "unbounded".
set -uo pipefail
ASLDIR=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64
TIMEOUT_S="${TIMEOUT_S:-25}"
HERE="$(cd "$(dirname "$0")" && pwd)"
f="$1"
base="${f%.asm}"
cd "$HERE"
rm -f "$base.p" "$base.lst"
AS_MSGPATH="$ASLDIR" timeout "$TIMEOUT_S" "$ASLDIR/asl" -xx -n -q -A -L -U -i "$HERE" "$f"
echo "ASL_EXIT=$?"
echo "=== LISTING ==="
cat "$base.lst" 2>/dev/null
