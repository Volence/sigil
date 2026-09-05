#!/usr/bin/env bash
# The DEPTH bound for LEGITIMATE nesting: N distinct files, each including the
# next, none repeated. p2/p3 measure the bound on a CYCLE; this measures it on a
# chain that no cycle rule should ever touch, so the two numbers can be compared
# and asl's limit shown to be about depth rather than about repetition.
#
#   ./depth.sh <N> [scratch-dir]
#
# Writes into a scratch dir under /home, never /tmp (tmpfs), so the committed
# probe directory does not fill with generated files.
set -uo pipefail
N="${1:?usage: depth.sh <N> [dir]}"
DIR="${2:-$(mktemp -d /home/volence/sonic_hacks/.parcel-include-scratch/depth.XXXXXX)}"
ASLDIR=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64
mkdir -p "$DIR"
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
rm -f top.lst top.p
AS_MSGPATH="$ASLDIR" timeout 60 "$ASLDIR/asl" -xx -n -q -A -L -U -i "$DIR" top.asm > asl.out 2>&1
rc=$?
head -6 asl.out
echo "N=$N ASL_EXIT=$rc"
echo "deepest level reached: $(grep -oE '^\([0-9]+\)' top.lst 2>/dev/null | tr -d '()' | sort -n | tail -1)"
echo "scratch: $DIR"
