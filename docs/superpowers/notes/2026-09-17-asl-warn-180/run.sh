#!/bin/sh
# run.sh [<probe>...] : assemble each probe (every `*.asm` here when none named) with the REFERENCE asl and print its
# exit status, pass-loop completeness, and every diagnostic it raised.
#
# The assembler is selected by digest through `../asl-reference/asl_ref.sh`;
# `asl_run` refuses a run that exited non zero. A probe here is evidence only
# if it prints `ASL_EXIT=0` and `ASL_DIAG=complete`: one error anywhere stops
# the pass loop, so every other line's diagnostics would be incomplete.
#
# The probes are assembled from a copy under SCRATCH (default
# /home/volence/sonic_hacks/.scratch/asl-warn-parity/run), so listings and
# object files never land in the tree. asl names its listing after the source
# path up to the FIRST dot, so the copy is assembled by a relative name from its
# own directory.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
SCRATCH="${SCRATCH:-/home/volence/sonic_hacks/.scratch/asl-warn-parity/run}"
mkdir -p "$SCRATCH" || exit 1
for inc in "$HERE"/*.inc; do
    [ -f "$inc" ] && cp "$inc" "$SCRATCH/"
done
rc_all=0
if [ "$#" -eq 0 ]; then
    set -- $(cd "$HERE" && ls ./*.asm | sed 's|^\./||')
fi
echo "probes: $#"
for p in "$@"; do
    p="${p%.asm}"
    cp "$HERE/$p.asm" "$SCRATCH/$p.asm" || exit 1
    (
        cd "$SCRATCH" || exit 1
        rm -f "$p.p" "$p.lst"
        echo "=== $p"
        out=$(USEANSI=n asl_run -xx -n -q -A -L -U "$p.asm" 2>&1)
        rc=$?
        printf '%s\n' "$out" | grep -E 'ASL_EXIT|ASL_DIAG|warning|error|REFUSED'
        # One row per warning: the source line number and that line's text.
        printf '%s\n' "$out" | grep -oE "$p\.asm\([0-9]+\)[^:]*: warning #[0-9]+" |
            sed -E 's/^[^(]*\(([0-9]+)\)[^:]*: warning #([0-9]+)/\1 \2/' |
            while read -r ln code; do
                printf '  FIRES #%s line %s: %s\n' "$code" "$ln" \
                    "$(sed -n "${ln}p" "$p.asm" | tr -s '\t ' ' ')"
            done
        exit $rc
    ) || rc_all=1
done
exit $rc_all
