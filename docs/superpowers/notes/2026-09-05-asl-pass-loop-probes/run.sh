#!/usr/bin/env bash
# run.sh [probe.asm ...] - assemble the probes beside this file and print, for
# each, the diagnostics asl gave and the footer facts that say whether it looked
# for all of them. With no arguments it runs every .asm here.
#
# The assembly goes through `asl_run`, never through "$ASL" directly, so the
# transcript carries ASL_EXIT and ASL_DIAG whether or not a reader looks. The
# console summary asl itself prints does NOT carry the pass-loop warning: it is
# in the -L listing footer and nowhere else, which is why -L is not optional
# here.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?

files=("$@")
if [ "${#files[@]}" -eq 0 ]; then
    for p in "$HERE"/*.asm; do files+=("$(basename "$p")"); done
fi

# BARE NAMES, from `$HERE` as the working directory, and this is not style.
# asl truncates the listing name at the FIRST dot in the path it is handed, so
# an absolute path through this checkout's `.claude/worktrees/...` writes the
# listing to the repository root and leaves none here. Everything below would
# then read a listing that is not there and report the pass state as unknown.
cd "$HERE" || exit 1

for f in "${files[@]}"; do
    base="$(basename "$f" .asm)"
    echo "===== $base"
    asl_run -xx -n -q -A -L -U -i . "$base.asm" 2>&1
    lst="$HERE/$base.lst"
    # Footer facts, read from the listing rather than described.
    grep -E '^[[:space:]]+[0-9]+ (passe?s?|errors?|warnings?)$' "$lst" 2>/dev/null \
        | sed 's/^[[:space:]]*/  FOOTER /'
    grep -cE 'error #1010' "$lst" 2>/dev/null \
        | sed 's/^/  UNDEFINED SYMBOLS REPORTED: /'
    echo
done
