#!/usr/bin/env bash
# SIBLING includes, not nested ones: one root that includes N files in sequence,
# none of which includes anything. Depth never exceeds 1.
#
#   ./siblings.sh <N> [sigil-binary]
#
# This is the probe that separates "depth" from "count". asl's bound is on how
# many includes are OPEN AT ONCE, so N sibling includes must be clean for any N —
# and an implementation that increments a depth counter and forgets to decrement
# it passes every nested test and fails here at N=200. That mutation (`m5`)
# survived the first version of `as_include_repeat.rs` untouched.
set -uo pipefail
N="${1:?usage: siblings.sh <N> [sigil-binary]}"
SIGIL="${2:-}"
ASLDIR=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64
DIR="$(mktemp -d /home/volence/sonic_hacks/.parcel-include-scratch/sib.XXXXXX)"
{
    printf '\tcpu\t68000\n'
    printf '\tpadding\toff\n'
    printf '\torg\t$1000\n'
    for ((i = 1; i <= N; i++)); do printf '\tinclude\t"s%d.inc"\n' "$i"; done
} > "$DIR/top.asm"
for ((i = 1; i <= N; i++)); do printf '\tdc.b\t$AA\n' > "$DIR/s$i.inc"; done
cd "$DIR" || exit 2
AS_MSGPATH="$ASLDIR" timeout 60 "$ASLDIR/asl" -xx -n -q -A -L -U -i "$DIR" top.asm > asl.out 2>&1
echo "asl  N=$N ASL_EXIT=$?  bytes=$(grep -c ': AA ' top.lst 2>/dev/null)"
head -3 asl.out
if [[ -n $SIGIL ]]; then
    timeout 60 "$SIGIL" top.asm -o out.bin > sigil.out 2> sigil.err
    rc=$?
    echo "sigil N=$N SIGIL_EXIT=$rc  bytes=$( [[ -f out.bin ]] && tail -c +4097 out.bin | wc -c || echo '(no image)')"
    head -3 sigil.err
fi
echo "scratch: $DIR"
