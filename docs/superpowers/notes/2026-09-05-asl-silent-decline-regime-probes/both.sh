#!/usr/bin/env bash
# Put every `r*.asm` probe here to EVERY runnable asl build in this workspace,
# N times each, and print the emitted code lines and diagnostics per run.
#
#   ./both.sh [N]        # N runs per probe per build, default 4
#
# Every build is named by MD5 above its block, and its banner is printed beside
# it so the two can be compared. NONE IS GUARDED, and that is the point:
# comparing them is the whole job here, and the defect the `../asl-reference/`
# guard answers to was never a second build — it was an UNIDENTIFIED instrument.
#
# THE BANNER NAMES NOTHING, and the population is larger than the sibling
# `asl-reference/README.md` table says. Six `asl` binaries in this workspace run
# on this machine and print `Macro Assembler 1.42 Beta [Bld 212]` verbatim,
# under FOUR distinct digests. Neither the second banner line nor the
# DIRECTORY NAME is a substitute for the digest either:
# `s2disasm/build_tools/Linux-x86/asl` is an x86-64 binary in the 32-bit slot,
# with a distinct digest from the one in `Linux-x86_64`, and it announces itself
# `(x86_64-Linux)`.
#
# Repeating N times is not superstition. Two of the four builds answer any
# operand they declined to value from an uninitialized word, so ONE run of an
# unstable row looks exactly like a stable one; the other two answer the same
# operand with the last value they computed, so a stable row is not evidence of
# an answer either. A STABLE VALUE IS NOT AN ANSWER.
#
# A build that is present but does not run on this machine is reported
# UNMEASURABLE. It is never silently skipped and never counted as agreement.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
N="${1:-4}"
BUILDS=(
    /home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64
    /home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64
    /home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86
    /home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86
)
cd "$HERE" || exit 2
for build in "${BUILDS[@]}"; do
    if [ ! -x "$build/asl" ]; then
        echo "# BUILD $build/asl UNMEASURABLE: not executable — reported, not skipped"
        continue
    fi
    banner=$(AS_MSGPATH="$build" "$build/asl" 2>&1 | head -2 | tr '\n' ' ')
    if [ -z "$banner" ]; then
        echo "# BUILD $build/asl UNMEASURABLE: does not run here — reported, not skipped"
        continue
    fi
    echo "############################################################"
    echo "# BUILD $build/asl"
    echo "#   md5    $(md5sum "$build/asl" | cut -d' ' -f1)"
    echo "#   banner $banner"
    echo "#   N=$N"
    for f in r*.asm; do
        base="${f%.asm}"
        for i in $(seq 1 "$N"); do
            rm -f "$base.p" "$base.lst"
            out=$(AS_MSGPATH="$build" "$build/asl" -xx -n -q -A -L -U -i "$HERE" "$f" 2>&1)
            rc=$?
            echo "--- $base run$i exit=$rc"
            sed -n '/Symbol Table/q;p' "$base.lst" 2>/dev/null \
                | grep -E '^ *[0-9]+/ +[0-9A-F]+ :' | sed 's/[[:space:]]*$//'
            printf '%s\n' "$out" | grep -E '> > >|Assertion'
        done
    done
done
rm -f r*.p r*.lst
echo "=== ALL BUILDS DONE ==="
