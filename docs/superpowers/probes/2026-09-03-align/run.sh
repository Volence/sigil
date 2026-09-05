#!/bin/sh
# Run one align probe through asl and print its listing.
#   run.sh <probe-basename> [asl-dir]
#
# THE DEFAULT ASSEMBLER IS SELECTED BY DIGEST, NOT BY BANNER — see
# `../../notes/asl-reference/`. The predecessor defaulted to the s2disasm
# flamewing build, which is the one that substitutes an uninitialized word for
# any operand it declined to give a value; it and the reference build print the
# same version string, so nothing in a listing said which had run.
#
# The [asl-dir] argument SURVIVES, because cross-checking a second build is what
# this directory is for and the defect was never the second build — it was an
# UNIDENTIFIED instrument. So an explicit dir bypasses the digest pin and prints
# the md5 of whatever it was handed, on its own line, above the listing. A run
# with no argument is the reference build and says so.
set -e
PROBE="$1"
HERE=$(cd "$(dirname "$0")" && pwd)
if [ $# -ge 2 ]; then
    ASLDIR="$2"
    [ -x "$ASLDIR/asl" ] || { echo "FATAL: no executable asl at $ASLDIR/asl" >&2; exit 2; }
    export AS_MSGPATH="$ASLDIR"
    echo "# CROSS-CHECK BUILD: $ASLDIR/asl md5 $(md5sum "$ASLDIR/asl" | cut -d' ' -f1)"
else
    . "$HERE/../../notes/asl-reference/asl_ref.sh" || exit $?
    echo "# reference build: $ASLDIR/asl md5 $(md5sum "$ASLDIR/asl" | cut -d' ' -f1)"
fi
rm -f "$PROBE.p" "$PROBE.lst"
"$ASLDIR/asl" -xx -n -q -A -L -U -i . "$PROBE.asm" 2>&1 || true
echo "----- listing -----"
sed -n '1,200p' "$PROBE.lst" 2>/dev/null
