#!/usr/bin/env bash
# Run every probe through the reference asl (selected by md5 via asl_ref.sh,
# assembled through asl_run, flattened with the p2bin beside it) and through a
# named sigil binary. One summary row per shape, plus per-shape transcripts.
#
#   matrix.sh <probe-dir> <sigil-binary> <results-dir>
set -uo pipefail
PROBES="$(cd "$1" && pwd)"
SIGIL="$2"
RES="$3"
mkdir -p "$RES"
RES="$(cd "$RES" && pwd)"
# The guard lives beside this directory's parent in the repo; ASL_REF overrides.
. "${ASL_REF:-$(cd "$(dirname "$0")" && pwd)/../asl-reference/asl_ref.sh}" || exit $?
P2BIN="$ASLDIR/p2bin"
echo "asl md5 $(md5sum "$ASL" | cut -d' ' -f1)  sigil $("$SIGIL" --version 2>&1 | head -1)" > "$RES/SUMMARY.tsv"
printf 'shape\tASL_EXIT\tasl_diag\tasl_first_error\tasl_hex\tSIGIL_EXIT\tsigil_first_error\tsigil_hex\n' >> "$RES/SUMMARY.tsv"
cd "$PROBES" || exit 9
n=0
for f in *.asm; do
    b="${f%.asm}"
    rm -f "$b.p" "$b.lst" "$b.bin"
    asl_run -xx -n -q -A -L -U -i . "$f" > "$RES/$b.asl.out" 2> "$RES/$b.asl.err"
    arc=$?
    diag="$(asl_diag_state "$b.lst")"
    cp -f "$b.lst" "$RES/$b.lst" 2>/dev/null
    aerr="$(/usr/bin/grep -E '> > > .*(error|warning)' "$b.lst" 2>/dev/null | head -1 | sed -E 's/^ *> > > //')"
    ahex="-"
    if [ "$arc" -eq 0 ] && [ -f "$b.p" ]; then
        "$P2BIN" "$b.p" "$b.bin" > /dev/null 2>&1
        ahex="$(od -An -tx1 -v "$b.bin" | tr -d ' \n')"
    fi
    "$SIGIL" "$f" --hex > "$RES/$b.sigil.out" 2> "$RES/$b.sigil.err"
    src=$?
    serr="$(/usr/bin/grep -m1 -E 'error|warning' "$RES/$b.sigil.err" "$RES/$b.sigil.out" | head -1 | sed -E 's/^[^:]*:(error|warning)/\1/')"
    shex="$(tr -d ' \n' < "$RES/$b.sigil.out")"
    [ "$src" -ne 0 ] && shex="-"
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$b" "$arc" "$diag" "${aerr:--}" "$ahex" "$src" "${serr:--}" "$shex" >> "$RES/SUMMARY.tsv"
    rm -f "$b.p" "$b.lst" "$b.bin"
    n=$((n+1))
done
echo "SHAPES_RUN=$n" >> "$RES/SUMMARY.tsv"
echo "MATRIX_END_MARKER" >> "$RES/SUMMARY.tsv"
