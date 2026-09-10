#!/bin/bash
# Runner for the UXa F5 partial-error-list probes.
# usage: run.sh <file.asm>  -- assembles with the sigil under test and prints both streams.
#
# SIGIL selects the binary. It defaults to the repository's release build rather
# than to whatever `sigil` is on PATH, because a stale binary and a current one
# print different error lists here and the path cannot tell them apart. Output
# goes to /dev/null: every probe in this directory is expected to FAIL, and none
# of them is a source of bytes.
#
# `|| exit $?` is load-bearing because `set -u` is not `set -e`.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../../.." && pwd)"
SIGIL="${SIGIL:-$ROOT/target/release/sigil}"
[ -x "$SIGIL" ] || { echo "no sigil at $SIGIL; set SIGIL or cargo build --release -p sigil-cli" >&2; exit 9; }
F="$1"
cd "$(dirname "$F")" || exit 9
"$SIGIL" "$(basename "$F")" -o /dev/null
st=$?
echo "sigil status=$st"
exit $st
