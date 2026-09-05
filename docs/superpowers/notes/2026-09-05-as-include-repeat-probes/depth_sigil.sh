#!/usr/bin/env bash
# The sigil half of depth.sh: the same generated chain of N distinct files, put
# through sigil, so the boundary is read off a run rather than off the constant
# in the source. The pair that matters is N=199 (both tools clean) and N=200
# (both tools refuse) — a bound checked only on one side of itself is not a
# bound.
#
#   ./depth_sigil.sh <sigil-binary> <N>
set -uo pipefail
SIGIL="${1:?usage: depth_sigil.sh <sigil-binary> <N>}"
N="${2:?usage: depth_sigil.sh <sigil-binary> <N>}"
DIR="$(mktemp -d /home/volence/sonic_hacks/.parcel-include-scratch/sdepth.XXXXXX)"
{
    printf '\tcpu\t68000\n'
    printf '\torg\t$1000\n'
    printf '\tinclude\t"n1.inc"\n'
} > "$DIR/top.asm"
for ((i = 1; i <= N; i++)); do
    {
        printf '\tdc.b\t$AA\n'
        if ((i < N)); then printf '\tinclude\t"n%d.inc"\n' "$((i + 1))"; fi
    } > "$DIR/n$i.inc"
done
cd "$DIR" || exit 2
timeout 60 "$SIGIL" top.asm -o out.bin > sigil.out 2> sigil.err
rc=$?
head -3 sigil.err
echo "N=$N SIGIL_EXIT=$rc"
if [[ -f out.bin ]]; then
    echo "size=$(stat -c%s out.bin)  \$AA bytes emitted=$(tail -c +4097 out.bin | tr -d '\000' | wc -c)"
else
    echo "(no image)"
fi
echo "scratch: $DIR"
