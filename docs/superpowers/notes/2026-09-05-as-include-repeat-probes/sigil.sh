#!/usr/bin/env bash
# The twin of run.sh: assemble each probe with SIGIL instead of asl, so the two
# columns of the matrix in ../2026-09-05-as-include-repeat.md are read off runs
# rather than off reasoning. Takes the sigil binary as $1; with no probe names it
# runs the whole set.
#
#   ./sigil.sh <sigil-binary> [p1.asm …]
#
# Every probe sits at `org $1000` and sigil emits an image from address 0, so
# the interesting bytes are the tail. This prints the diagnostic stream, the
# image SIZE, and the bytes FROM $1000 — the size and the bytes are both
# discriminators, and quoting only one of them is how a once-vs-twice difference
# hides (a second copy of a header at a different address can leave a prefix
# identical).
#
# `timeout` for the same reason run.sh has one: p2/p3 are the unbounded shapes.
# A 124 is a measurement, not a flake.
set -uo pipefail
SIGIL="$1"; shift
HERE="$(cd "$(dirname "$0")" && pwd)"
TIMEOUT_S="${TIMEOUT_S:-25}"
OUT="${SIGIL_PROBE_OUT:-$(mktemp -d /home/volence/sonic_hacks/.parcel-include-scratch/probe.XXXXXX)}"
mkdir -p "$OUT"
if (($# == 0)); then set -- p1.asm p2.asm p3.asm p4.asm p5.asm p6.asm p7.asm; fi
for f in "$@"; do
    b="${f%.asm}"
    echo "=== $f ==="
    ( cd "$HERE" && timeout "$TIMEOUT_S" "$SIGIL" "$f" -o "$OUT/$b.bin" 2>&1 )
    echo "SIGIL_EXIT=$?"
    if [[ -f $OUT/$b.bin ]]; then
        printf 'size=%s  bytes from $1000: ' "$(stat -c%s "$OUT/$b.bin")"
        tail -c +4097 "$OUT/$b.bin" | xxd -p | tr -d '\n'
        echo
    else
        echo "(no image)"
    fi
done
echo "images: $OUT"
