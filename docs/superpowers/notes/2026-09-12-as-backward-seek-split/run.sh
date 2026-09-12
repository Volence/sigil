#!/usr/bin/env bash
# Run every probe through the reference asl (md5-pinned by asl_ref.sh, invoked
# through asl_run, flattened by the p2bin beside it) and through a named sigil
# binary, and write one transcript set per shape plus RAW.tsv.
#
#   run.sh <sigil-binary> <results-dir>
#
# asl bytes are taken ONLY from runs that exited 0. A non-zero asl exit records
# "-" in the hex column, never the partial image.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SIGIL="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
RES="$2"
mkdir -p "$RES"
RES="$(cd "$RES" && pwd)"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
P2BIN="$ASLDIR/p2bin"
{
    echo "asl md5 $(md5sum "$ASL" | cut -d' ' -f1)"
    echo "p2bin md5 $(md5sum "$P2BIN" | cut -d' ' -f1)"
    echo "sigil md5 $(md5sum "$SIGIL" | cut -d' ' -f1)  $("$SIGIL" --version 2>&1 | head -1)"
} > "$RES/PROVENANCE.txt"
printf 'shape\tASL_EXIT\tasl_diag\tasl_hex\tSIGIL_EXIT\tsigil_hex\n' > "$RES/RAW.tsv"
cd "$HERE/probes" || exit 9
n=0
for f in *.asm; do
    b="${f%.asm}"
    rm -f "$b.p" "$b.lst" "$b.bin"
    # Relative source path from the probe's own directory: asl truncates the
    # listing name at the FIRST dot of the path it is given (asl_ref.sh).
    asl_run -xx -n -q -A -L -U -i . "$f" > "$RES/$b.asl.out" 2> "$RES/$b.asl.err"
    arc=$?
    diag="$(asl_diag_state "$b.lst")"
    if [ -f "$b.lst" ]; then cp -f "$b.lst" "$RES/$b.lst"; fi
    ahex="-"
    if [ "$arc" -eq 0 ] && [ -f "$b.p" ]; then
        "$P2BIN" "$b.p" "$b.bin" > "$RES/$b.p2bin.out" 2>&1
        echo "P2BIN_EXIT=$?" >> "$RES/$b.p2bin.out"
        ahex="$(od -An -tx1 -v "$b.bin" | tr -d ' \n')"
    fi
    "$SIGIL" "$f" --hex > "$RES/$b.sigil.out" 2> "$RES/$b.sigil.err"
    src=$?
    echo "SIGIL_EXIT=$src" >> "$RES/$b.sigil.err"
    # `--hex` prints the image as hex-pair lines, then a `built: N bytes` line
    # on the same stream. Keep only the hex-pair lines.
    shex="$(/usr/bin/grep -E '^[0-9A-Fa-f]{2}( [0-9A-Fa-f]{2})*$' "$RES/$b.sigil.out" | tr -d ' \n' | tr 'A-F' 'a-f')"
    [ "$src" -ne 0 ] && shex="-"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$b" "$arc" "$diag" "$ahex" "$src" "$shex" >> "$RES/RAW.tsv"
    rm -f "$b.p" "$b.lst" "$b.bin"
    n=$((n+1))
done
echo "SHAPES_RUN=$n" >> "$RES/RAW.tsv"
echo "RUN_END_MARKER" >> "$RES/RAW.tsv"
