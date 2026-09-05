#!/usr/bin/env bash
# Put every probe here to BOTH shipped asl builds, N times each, and print the
# code lines and diagnostics side by side. This is the runner that decides which
# of the parent note's rows are build-independent and which are not.
#
#   ./both.sh [N]        # N runs per probe per build, default 4
#
# Both builds are named by MD5 above their block. Neither is guarded, and that
# is the point: comparing them is the whole job, and the defect this directory
# answers to was never a second build — it was an UNIDENTIFIED instrument.
#
# Repeating N times is not superstition. The s2disasm build answers any operand
# it declined to value from an uninitialized word, so a single run of an
# unstable row looks exactly like a stable one.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
N="${1:-4}"
S1=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64
S2=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64
cd "$HERE" || exit 2
for build in "$S1" "$S2"; do
    [ -x "$build/asl" ] || { echo "UNMEASURABLE: no asl at $build/asl — reported, not skipped" >&2; continue; }
    echo "############################################################"
    echo "# BUILD $build/asl md5 $(md5sum "$build/asl" | cut -d' ' -f1)  N=$N"
    for f in d*.asm; do
        base="${f%.asm}"
        for i in $(seq 1 "$N"); do
            rm -f "$base.p" "$base.lst"
            out=$(AS_MSGPATH="$build" "$build/asl" -xx -n -q -A -L -U -i "$HERE" "$f" 2>&1)
            rc=$?
            echo "--- $base run$i exit=$rc"
            grep -E '^ *[0-9]+/ +[0-9A-F]+ : [0-9A-F]' "$base.lst" 2>/dev/null | sed 's/[[:space:]]*$//'
            printf '%s' "$out" | grep '> > >'
        done
    done
done
rm -f d*.p d*.lst
echo "=== BOTH BUILDS DONE ==="
