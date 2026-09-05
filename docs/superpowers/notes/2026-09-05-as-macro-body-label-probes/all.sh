#!/usr/bin/env bash
# Run every probe in this directory through the reference asl and print the
# whole diagnostic stream plus the listing. `./all.sh` prints; `./all.sh --hash`
# prints only the md5 of the stream, which is how the three-run stability check
# in ../2026-09-05-as-macro-body-label.md was taken.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
for f in "$HERE"/p*.asm; do
    b="$(basename "$f")"
    echo "########## $b ##########"
    "$HERE/run.sh" "$b" 2>&1
done
