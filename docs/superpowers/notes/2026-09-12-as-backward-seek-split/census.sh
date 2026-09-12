#!/usr/bin/env bash
# Exposure census: run an INSTRUMENTED sigil (instrument.py over a git-archive
# copy) over each prepared corpus copy and count the three instrument lines.
#
#   census.sh <instrumented-sigil> <out-dir> <name>=<corpus-dir>:<root-file> ...
#
# Every corpus run ends in errors (their own open rows), which is expected:
# the front end still evaluates every line it reaches. The instrument's own
# positive control is the probes directory, run first as `probes`.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SIGIL="$1"; OUT="$2"; shift 2
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
echo "instrumented sigil md5 $(md5sum "$SIGIL" | cut -d' ' -f1)" > "$OUT/CENSUS.txt"
count() { /usr/bin/grep -c -- "$1" "$2"; }
# Positive control: every probe, one run each.
: > "$OUT/probes.err"
for f in "$HERE"/probes/*.asm; do
    (cd "$HERE/probes" && "$SIGIL" "$(basename "$f")" >/dev/null 2>>"$OUT/probes.err")
done
for spec in "probes=" "$@"; do
    name="${spec%%=*}"
    if [ "$name" != probes ]; then
        rest="${spec#*=}"; dir="${rest%%:*}"; rootf="${rest#*:}"
        (cd "$dir" && "$SIGIL" "$rootf" > "$OUT/$name.out" 2> "$OUT/$name.err")
        echo "$name: exit $?, root $rootf" >> "$OUT/CENSUS.txt"
    fi
    e="$OUT/$name.err"
    printf '%s\tSEEKFEED=%s\tSEEKSPLIT=%s\tSEEKSPLIT-BY-ORG=%s\tSEEKSPLIT-NOSEEK=%s\tstderr_lines=%s\n' \
        "$name" "$(count 'SEEKFEED' "$e")" "$(count 'SEEKSPLIT close' "$e")" \
        "$(count 'SEEKSPLIT-BY-ORG' "$e")" "$(count 'SEEKSPLIT-NOSEEK' "$e")" \
        "$(wc -l < "$e")" >> "$OUT/CENSUS.txt"
done
echo "CENSUS_END_MARKER" >> "$OUT/CENSUS.txt"
