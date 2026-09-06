#!/usr/bin/env bash
# sweep.sh [repo-root] - derive the population of committed .asm files whose asl
# run does not carry a complete diagnostic set. Prints one TSV row per tracked
# .asm: state, exit status, pass footer, error footer, path.
#
# HOW IT RUNS THEM, and this is the limit to read before the numbers. Each file
# is assembled STANDALONE, from a copy of its own directory, with the blessed
# flags. That is what most of this tree's runners do, but not all: a probe that
# needs an include path into one of the disassembly corpora, or that is an
# include FRAGMENT rather than a probe, fails here for a reason its own runner
# would not produce. Such rows show as `nofooter` with an `error in opening
# file`, and they are sweep artifacts, not findings. Check the diagnostic before
# reading a row as a result.
#
# THE OUTPUT IS BUILT IN A SCRATCH COPY, never in the tracked tree, because asl
# writes a .lst and a .p beside its source.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="${1:-$(cd "$HERE/../../../.." && pwd)}"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$ROOT" || exit 1

git ls-files '*.asm' | while IFS= read -r rel; do
    dir="$(dirname "$rel")"; base="$(basename "$rel")"; stem="${base%.asm}"
    mkdir -p "$WORK/$dir"
    cp -r "$ROOT/$dir/." "$WORK/$dir/" 2>/dev/null
    # A BARE NAME from the file's own directory, deliberately: asl truncates the
    # listing name at the FIRST dot in the path it is handed, so an absolute
    # path through a `.claude/worktrees/...` checkout writes the listing outside
    # the tree and this sweep would read `nofooter` for every single file.
    ( cd "$WORK/$dir" && "$ASL" -xx -n -q -A -L -U -i . "$base" >/dev/null 2>&1 )
    rc=$?
    lst="$WORK/$dir/$stem.lst"
    pline=""; eline=""
    if [ -f "$lst" ]; then
        pline=$(grep -E '^ +[0-9]+ passe?s?$' "$lst" | tail -1 | tr -s ' ')
        eline=$(grep -E '^ +[0-9]+ errors?$' "$lst" | tail -1 | tr -s ' ')
    fi
    state="$(asl_diag_state "$lst")"
    [ "$state" = complete ] && [ -z "$pline" ] && state=nofooter
    printf '%s\t%s\t%s\t%s\t%s\n' "$state" "$rc" "${pline# }" "${eline# }" "$rel"
done
