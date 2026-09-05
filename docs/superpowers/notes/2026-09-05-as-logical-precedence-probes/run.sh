#!/usr/bin/env bash
# Probe runner: assemble one file with the reference asl and print the listing.
# Same shape as 2026-09-05-as-macro-body-label-probes/run.sh. Not part of the suite.
# The `|| exit $?` is load-bearing: `set -uo pipefail` is not `set -e`, so a
# sourced guard that only returns non-zero would stop nothing.
#
# The assembly goes through `asl_run`, not through `"$ASLDIR/asl"` directly.
# Printing `ASL_EXIT=2` and then dumping the listing anyway, which is what this
# runner used to do, puts the status in the transcript and still lets a reader
# quote a byte column an error changed. `asl_run` refuses out loud, immediately
# before the listing. See `../asl-reference/partial_failure.asm`.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
f="$1"
base="${f%.asm}"
cd "$HERE"
rm -f "$base.p" "$base.lst"
asl_run -xx -n -q -A -L -U -i "$HERE" "$f"
# `asl_run` reports the status and any refusal on STDERR; this keeps `ASL_EXIT=`
# on STDOUT too, matching the sibling runner this one is a copy of.
echo "ASL_EXIT=$?"
echo "=== LISTING ==="
cat "$base.lst" 2>/dev/null
