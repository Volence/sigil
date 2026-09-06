#!/usr/bin/env bash
# Assemble one probe in THIS directory with the reference asl, through the
# blessed `asl_run` wrapper, and print the listing.
#
# Run it from this directory with a BARE filename: asl truncates the listing
# name at the FIRST dot in the path it is handed, and every path to this file
# runs through `.claude/worktrees/...`, so an absolute path writes the listing
# outside the worktree and this script would read a stale one.
set -u
cd "$(dirname "$0")" || exit 2
. ../asl-reference/asl_ref.sh || exit $?
src="$1"
rm -f "${src%%.*}.lst" "${src%%.*}.p"
asl_run -xx -n -q -A -L -U -i . "$src"
rc=$?
echo "----- listing: ${src%%.*}.lst"
cat "${src%%.*}.lst"
exit "$rc"
