#!/usr/bin/env bash
# Probe runner: assemble one file with the reference asl and print the listing.
# Same shape as 2026-09-05-as-macro-body-label-probes/run.sh. Not part of the suite.
# The `|| exit $?` is load-bearing: `set -uo pipefail` is not `set -e`, so a
# sourced guard that only returns non-zero would stop nothing.
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
