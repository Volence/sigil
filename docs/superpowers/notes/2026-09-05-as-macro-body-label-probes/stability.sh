#!/usr/bin/env bash
# Oracle stability: asl 1.42 is known to answer differently across runs for at
# least one shape, so every probe this parcel relies on is run THREE times and
# the whole stream hashed. Three identical hashes, or the shape is excluded from
# the note and said so.
#
# ── THE CLOCK COMES OUT FIRST, AND THAT RULE NOW LIVES IN ONE PLACE ──────────
# asl stamps the wall clock into the stream in three places, and a clock reading
# is not a property of the program being assembled: a batch straddling a tick
# reports the assembler disagreeing with itself when only the clock moved. The
# blanking rules used to be written out inline here; they are now
# `../asl-declock/declock.sed`, which carries the measured shape of every stamp
# and, beside it, a `selfcheck.sh` that proves the filter still separates two
# streams differing in CONTENT. Point the next stability runner at the same file
# instead of re-deriving the rules — two of the three were wrong when they were
# re-derived here, and both were invisible from inside this script.
#
# THE HASHES THIS PRINTS DIFFER FROM THE TABLE IN
# `../2026-09-05-as-macro-body-label.md`, which was taken before two repairs to
# the filter: the time rule left the `AM`/`PM` meridiem standing (so a batch
# straddling noon or midnight false-alarmed on four banner lines), and the
# duration anchor accepted only `N.NN seconds` when asl prints
# `1 minute, 17.08 seconds assembly time` past sixty. Neither repair can hide a
# divergence — both only remove a clock reading — but both move every hash.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
DECLOCK="$HERE/../asl-declock/declock.sed"
[[ -f $DECLOCK ]] || { echo "FATAL: no declock filter at $DECLOCK" >&2; exit 2; }
echo "# filter $DECLOCK md5 $(md5sum "$DECLOCK" | cut -d' ' -f1)"
for f in "$HERE"/p*.asm; do
    b="$(basename "$f" .asm)"
    printf '%s ' "$b"
    for i in 1 2 3; do
        printf '%s ' "$("$HERE/run.sh" "$b.asm" 2>&1 \
            | sed -E -f "$DECLOCK" \
            | md5sum | cut -c1-12)"
    done
    echo
done
