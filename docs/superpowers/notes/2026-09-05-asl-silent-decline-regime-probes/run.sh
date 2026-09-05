#!/usr/bin/env bash
# Assemble one silent-decline-regime probe and print its code lines and any
# diagnostic.
#
#   ./run.sh r01_bare_reg.asm              # the reference build (digest-pinned)
#   ./run.sh r01_bare_reg.asm <asl-dir>    # a second build, md5 announced
#
# THE ASSEMBLER IS IDENTIFIED BY DIGEST, NOT BY BANNER — see `../asl-reference/`.
# Several `asl` binaries in this workspace print `Macro Assembler 1.42 Beta
# [Bld 212]` verbatim and are not the same program.
#
# `|| exit $?` is load-bearing — `set -uo pipefail` is not `set -e`.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
f="${1:?usage: run.sh <probe.asm> [asl-dir]}"
base="${f%.asm}"
if [ $# -ge 2 ]; then
    ASLDIR="$2"
    [ -x "$ASLDIR/asl" ] || { echo "FATAL: no executable asl at $ASLDIR/asl" >&2; exit 2; }
    export AS_MSGPATH="$ASLDIR"
    echo "# CROSS-CHECK BUILD: $ASLDIR/asl md5 $(md5sum "$ASLDIR/asl" | cut -d' ' -f1)"
else
    . "$HERE/../asl-reference/asl_ref.sh" || exit $?
    echo "# reference build: $ASLDIR/asl md5 $(md5sum "$ASLDIR/asl" | cut -d' ' -f1)"
fi
cd "$HERE" || exit 2
rm -f "$base.p" "$base.lst"
# Sonic 2's own flags minus the two that only redirect output.
"$ASLDIR/asl" -xx -n -q -A -L -U -i "$HERE" "$f"
echo "ASL_EXIT=$?"
# THE INCOMPLETE-MEASUREMENT TELL. A forward reference is legal in AS, so an
# undefined symbol is a provisional value in the first pass and is only reported
# when a later pass finds it still undefined. An error found EARLIER stops the
# pass loop, and every later `symbol undefined` is then emitted as a masked
# provisional value with NO diagnostic. asl says so itself, in the listing
# footer, and this is the line that says it. A listing carrying it has NOT
# measured what it looks like it measured. Proof it fires:
# `r11_earlier_error_swallows_undef.asm` prints it and reports neither of its two
# undefined symbols; `r11b_no_earlier_error.asm`, one line different, does not
# print it and reports both.
if grep -q 'Additional necessary passes not started' "$base.lst" 2>/dev/null; then
    echo "INCOMPLETE: asl stopped its pass loop on an earlier error —"
    echo "INCOMPLETE: later 'symbol undefined' reports are SUPPRESSED in this listing."
fi
echo "=== LISTING ==="
cat "$base.lst" 2>/dev/null
