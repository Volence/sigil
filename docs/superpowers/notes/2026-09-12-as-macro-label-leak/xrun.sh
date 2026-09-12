#!/usr/bin/env bash
# Run each x-probe through the reference asl; print exit, errors, and the
# nameless symbol rows (`__forwN`, `__backN`) from asl's symbol table.
set -uo pipefail
D="$1"
. "${ASL_REF:-$(cd "$(dirname "$0")" && pwd)/../asl-reference/asl_ref.sh}" || exit $?
cd "$D" || exit 9
for f in x*.asm; do
    b="${f%.asm}"
    rm -f "$b.p" "$b.lst"
    asl_run -xx -n -q -A -L -U -i . "$f" > /dev/null 2> "$b.err"
    rc=$?
    echo "=== $b  ASL_EXIT=$rc  $(asl_diag_state "$b.lst")"
    /usr/bin/grep -E '> > > .*error' "$b.lst" | head -3
    /usr/bin/grep -oE '[ *]__(forw|back)[0-9]+ :[ ]+[0-9A-F]+' "$b.lst"
done
