#!/usr/bin/env bash
# run.sh -- assemble one probe with the REFERENCE asl and print everything a
# reader needs to judge the run: exit status, ASL_DIAG (whether asl finished
# its pass loop and so whether it LOOKED for undefined symbols), and the
# listing.
#
# Run from this directory, with a RELATIVE source name: asl truncates the
# listing name at the FIRST dot of the path it is given, and this tree lives
# under `.claude/worktrees/...`, so an absolute path writes the listing at the
# repository root instead of here.
set -u
cd "$(dirname "$0")" || exit 2
. ../asl-reference/asl_ref.sh || exit $?

for src in "$@"; do
    base="${src%.asm}"
    rm -f "$base.lst" "$base.p"
    echo "════════ $src ════════"
    asl_run -xx -n -q -A -L -U -i . "$src"
    echo "ASL_RC=$?"
    echo "──────── listing $base.lst ────────"
    cat "$base.lst" 2>/dev/null || echo "(no listing)"
done
