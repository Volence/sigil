#!/usr/bin/env bash
# The MECHANISM behind the varying value: an uninitialized read of something the
# kernel randomises per process.
#
# `u_bare.asm` is `move.w #zz,d0` with `zz` undefined. Against the 0dee1f98 build
# of asl it emits a different word almost every run. Run the same probe under
# `setarch -R`, which disables address-space randomization, and the value
# collapses to a single constant — measured $5555. That is what distinguishes an
# uninitialized read from, say, a clock or a hash seed, and it is why no amount of
# re-minting makes such a value reproducible on a normal machine.
#
# Against the 61e672 build both columns are a constant 0, so running this script
# on the pinned assembler shows two identical columns and proves nothing; point
# ASLDIR at the build where the shape is live.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ASLDIR="${ASLDIR:-/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64}"
N="${1:-8}"
WORK="${TMPDIR:-/tmp}/asl_aslr.$$"
mkdir -p "$WORK"; trap 'rm -rf "$WORK"' EXIT
cp "$HERE/u_bare.asm" "$WORK"/

command -v setarch >/dev/null || { echo "UNMEASURABLE: no setarch on this host" >&2; exit 2; }
echo "# asl $ASLDIR/asl md5 $(md5sum "$ASLDIR/asl" | cut -d' ' -f1)  N=$N"

for mode in randomized norandom; do
    vals=""
    for i in $(seq 1 "$N"); do
        rm -f "$WORK/u_bare.p" "$WORK/u_bare.lst"
        if [[ $mode == norandom ]]; then
            ( cd "$WORK" && AS_MSGPATH="$ASLDIR" timeout 60 setarch -R "$ASLDIR/asl" -xx -n -q -A -L -U -i "$WORK" u_bare.asm ) >/dev/null 2>&1
        else
            ( cd "$WORK" && AS_MSGPATH="$ASLDIR" timeout 60 "$ASLDIR/asl" -xx -n -q -A -L -U -i "$WORK" u_bare.asm ) >/dev/null 2>&1
        fi
        vals="$vals $(grep -oE '303C [0-9A-F]{4}' "$WORK/u_bare.lst" 2>/dev/null | head -1 | tr -d ' ')"
    done
    printf '%-11s distinct=%-3s %s\n' "$mode" "$(printf '%s\n' $vals | sort -u | wc -l)" "$vals"
done
