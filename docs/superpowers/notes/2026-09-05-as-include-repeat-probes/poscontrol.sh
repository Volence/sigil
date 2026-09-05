#!/usr/bin/env bash
# THE POSITIVE CONTROL. Four aeon shapes holding their CRC32 across this parcel
# attests that the plumbing moves no byte. It attests NOTHING about the
# behaviour the parcel changed, because aeon's population of repeated includes
# is zero — the whole tree holds exactly TWO `include` lines, one per game
# (`games/sonic4/game_root.asm:35` and `games/demo/game_root.asm:32`, both
# `include "engine/debug/debugger.asm"`), and the census reads
# `executed=1 repeats=0` in every pass of every shape.
#
# So this script makes the population non-zero, in a COPY of the reference tree,
# and shows the two binaries disagreeing about it:
#
#   * the BRANCH-POINT binary silently drops the second include and builds a ROM
#     with the same CRC as the untouched tree — the divergence, reproduced on
#     real aeon source;
#   * the PARCEL binary executes it, and `debugger.asm` defines labels, so the
#     build is refused with `symbol double defined` exactly as asl refuses it.
#
#   ./poscontrol.sh <basepoint-binary> <parcel-binary> <aeon-dir> <scratch-dir>
#
# The scratch dir gets a full aeon copy. KEEP IT OUTSIDE the sigil worktree and
# outside any cargo target dir: a second `tools/suite_paths.py` under either
# makes `scripts_name_their_tree` and the drift harness answer COULD NOT MEASURE,
# a failure that looks nothing like its cause.
set -uo pipefail
BASE="$1"; PARCEL="$2"; AEON="$3"; SCRATCH="$4"
COPY="$SCRATCH/aeon-doubled"
rm -rf "$COPY"
mkdir -p "$SCRATCH"
cp -a "$AEON" "$COPY"

ROOT="$COPY/games/sonic4/game_root.asm"
before=$(grep -c 'include "engine/debug/debugger.asm"' "$ROOT")
# Duplicate the include line in place.
perl -0pi -e 's{(^[ \t]*include "engine/debug/debugger\.asm"\n)}{$1$1}m' "$ROOT"
after=$(grep -c 'include "engine/debug/debugger.asm"' "$ROOT")
echo "include lines in games/sonic4/game_root.asm: $before -> $after"
if [[ $after -ne $((before + 1)) ]]; then
    echo "REFUSING — the injection did not land. A control that did not change the"
    echo "tree measures the untouched tree and agrees with everything."
    exit 2
fi

crc() { python3 -c 'import sys,zlib;print("%08x"%(zlib.crc32(open(sys.argv[1],"rb").read())&0xffffffff))' "$1"; }

for pair in "basepoint:$BASE" "parcel:$PARCEL"; do
    tag="${pair%%:*}"; bin="${pair#*:}"
    echo "### $tag on the DOUBLED tree"
    SIGIL_CENSUS_INCLUDE=1 AEON_DIR="$COPY" "$bin" build --aeon "$COPY" --game sonic4 \
        -o "$SCRATCH/$tag.bin" > "$SCRATCH/$tag.out" 2> "$SCRATCH/$tag.err"
    echo "  BUILD_EXIT=$?"
    grep -m1 '^CENSUS-INCLUDE' "$SCRATCH/$tag.err" | sed 's/^/  /'
    if [[ -f $SCRATCH/$tag.bin ]]; then
        echo "  crc32=$(crc "$SCRATCH/$tag.bin")  size=$(stat -c%s "$SCRATCH/$tag.bin")"
    else
        echo "  (NO IMAGE)"
    fi
    grep -i 'error' "$SCRATCH/$tag.err" | grep -v CENSUS | head -3 | sed 's/^/  /'
done
echo
echo "The reference (UNDOUBLED) tree's sonic4 CRC is 1c09fbfc / 819131 bytes."
echo "A basepoint row matching that is the divergence: a source change that must"
echo "move the ROM, silently producing the identical ROM."
