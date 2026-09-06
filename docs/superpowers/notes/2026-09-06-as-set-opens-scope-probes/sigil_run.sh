#!/usr/bin/env bash
# sigil_run.sh — assemble each named probe with a sigil binary and print its
# bytes, its diagnostics and its exit status.
#
#   SIGIL=<path to sigil> ./sigil_run.sh probe.asm ...
#
# Defaults to this worktree's release build.  Run from this directory.
set -u
cd "$(dirname "$0")" || exit 2
SIGIL="${SIGIL:-../../../../.target-land/release/sigil}"
if [ ! -x "$SIGIL" ]; then
    echo "FATAL: no executable sigil at $SIGIL" >&2
    exit 2
fi
echo "SIGIL=$SIGIL  md5=$(md5sum "$SIGIL" | cut -d' ' -f1)"
for src in "$@"; do
    echo "════════ $src ════════"
    out="$("$SIGIL" "$src" --hex 2>&1)"
    rc=$?
    printf '%s\n' "$out"
    echo "SIGIL_RC=$rc"
done
