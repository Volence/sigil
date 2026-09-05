#!/bin/bash
# usage: run.sh <ref|asl-path> <file.asm> <outprefix>
#
# THE ASSEMBLER IS IDENTIFIED BY DIGEST, NOT BY BANNER — see `../asl-reference/`.
# Four `asl` binaries in this workspace print `Macro Assembler 1.42 Beta [Bld
# 212]` verbatim and are not the same program, and one of them answers any
# operand it declined to value from uninitialized memory. The predecessor took a
# bare path and printed nothing about it, so a log said only that "an asl" ran.
#
# `ref` selects the digest-pinned reference build via `asl_ref.sh`. An explicit
# PATH still works and is not refused, because this directory's claim is that
# BOTH shipped builds agree on every byte column here, and that agreement is a
# measurement to re-take rather than an assumption — the defect was never a
# second build, it was an UNIDENTIFIED instrument. Either way the md5 of the
# binary that ran is printed above the result, so no run is anonymous.
#
# The argument ORDER is unchanged from the predecessor on purpose: an old
# `run.sh <path> p1.asm out` still does what it always did, rather than silently
# assembling the assembler's own path as a source file.
HERE="$(cd "$(dirname "$0")" && pwd)"
WHICH="${1:?usage: run.sh <ref|asl-path> <file.asm> <outprefix>}"
SRC="$2"; OUT="$3"
if [ "$WHICH" = ref ]; then
    . "$HERE/../asl-reference/asl_ref.sh" || exit $?
    echo "# reference build: $ASL md5 $(md5sum "$ASL" | cut -d' ' -f1)"
else
    ASL="$WHICH"
    [ -x "$ASL" ] || { echo "FATAL: no executable asl at $ASL" >&2; exit 2; }
    export AS_MSGPATH="$(dirname "$ASL")"
    echo "# CROSS-CHECK BUILD: $ASL md5 $(md5sum "$ASL" | cut -d' ' -f1)"
fi
rm -f "$OUT.lst" "$OUT.p" "$OUT.log"
timeout 60 "$ASL" -xx -n -q -A -L -U -i . -olist "$OUT.lst" -o "$OUT.p" "$SRC" > "$OUT.log" 2>&1
echo "asl_exit=$?"
