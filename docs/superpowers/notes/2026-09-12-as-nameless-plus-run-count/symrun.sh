#!/usr/bin/env bash
# Run every probe in a directory through the reference asl and print, per
# shape: exit status, pass-loop state, the first errors, asl's nameless symbol
# rows (`__forwN` / `__backN`, zero-based, from its symbol table), and the
# p2bin image (only for an exit-0 run).
#
#   symrun.sh <probe-dir>
#
# Symbols filed in a macro expansion or a loop iteration are local to it and
# do NOT appear in the symbol table; only file-level ones do.
set -uo pipefail
D="$1"
. "${ASL_REF:-$(cd "$(dirname "$0")" && pwd)/../asl-reference/asl_ref.sh}" || exit $?
P2BIN="$ASLDIR/p2bin"
cd "$D" || exit 9
n=0
for f in *.asm; do
    b="${f%.asm}"
    rm -f "$b.p" "$b.lst" "$b.bin"
    asl_run -xx -n -q -A -L -U -i . "$f" > /dev/null 2> "$b.err"
    rc=$?
    echo "=== $b  ASL_EXIT=$rc  $(asl_diag_state "$b.lst")"
    /usr/bin/grep -E '> > > .*(error|warning)' "$b.lst" | head -3
    /usr/bin/grep -oE '[ *]__(forw|back)[0-9]+ :[ ]+[0-9A-F]+' "$b.lst" | sort -t w -k 2
    if [ "$rc" -eq 0 ] && [ -f "$b.p" ]; then
        "$P2BIN" "$b.p" "$b.bin" > /dev/null 2>&1
        # From $100 on: every probe is `org $100`, so the 256 bytes before
        # it are p2bin's zero fill, not the probe's.
        echo "hex@100 $(od -An -tx1 -v -j 256 "$b.bin" | tr -d ' \n')"
    fi
    rm -f "$b.p" "$b.bin"
    n=$((n+1))
done
echo "SHAPES_RUN=$n"
echo "SYMRUN_END_MARKER"
