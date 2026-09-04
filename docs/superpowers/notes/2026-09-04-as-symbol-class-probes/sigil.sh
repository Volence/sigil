#!/usr/bin/env bash
# The twin of run.sh: assemble each probe with SIGIL instead of asl, so the two
# columns of the matrix in ../2026-09-04-as-symbol-class-tracking.md are read off
# runs rather than off reasoning. Takes the sigil binary as $1. Not part of the
# suite.
set -uo pipefail
SIGIL="$1"; shift
HERE="$(cd "$(dirname "$0")" && pwd)"
for f in "$@"; do
    echo "=== $f ==="
    "$SIGIL" "$HERE/$f" --hex 2>&1
    echo "SIGIL_EXIT=$?"
done
