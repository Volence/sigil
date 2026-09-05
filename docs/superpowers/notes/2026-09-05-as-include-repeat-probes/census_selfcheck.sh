#!/usr/bin/env bash
# CALIBRATE the engagement counter against sources whose answer is known.
#
#   ./census_selfcheck.sh <sigil-binary>
#
# The counter is evidence in the parcel report — "an empty diff beside
# `repeats=0`" only means something if `repeats` would have been non-zero had
# there been repeats. Nothing in `as_include_repeat.rs` reads it (it is an
# instrument, not a rule), so this is where it is held honest: four probe
# sources whose repeated-include count can be read straight off the file.
#
#   p1  two includes of one header            -> 1 repeat
#   p4  diamond, `d` included from `b` and `c`-> 1 repeat
#   p5  three spellings of one path           -> 2 repeats
#   p7  three includes of one header          -> 2 repeats
#
# A counter that always reports zero (the `m6` mutation) fails every row here; a
# counter that counted every include rather than the repeats fails every row too,
# in the other direction. Exits non-zero on the first mismatch.
set -uo pipefail
SIGIL="${1:?usage: census_selfcheck.sh <sigil-binary>}"
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="$(mktemp -d /home/volence/sonic_hacks/.parcel-include-scratch/selfcheck.XXXXXX)"
rc=0
check() {
    local probe="$1" want_repeats="$2" want_executed="$3"
    ( cd "$HERE" && SIGIL_CENSUS_INCLUDE=1 "$SIGIL" "$probe.asm" -o "$OUT/$probe.bin" ) \
        > "$OUT/$probe.out" 2> "$OUT/$probe.err"
    # One CENSUS-INCLUDE line per pass; the AS front end converges over several
    # and every pass walks the same includes, so the per-pass numbers are equal
    # and the first line is the reading.
    local line
    line=$(grep -m1 '^CENSUS-INCLUDE' "$OUT/$probe.err")
    local got_r got_e
    got_r=$(sed -n 's/.*repeats=\([0-9]*\).*/\1/p' <<<"$line")
    got_e=$(sed -n 's/.*executed=\([0-9]*\).*/\1/p' <<<"$line")
    if [[ $got_r == "$want_repeats" && $got_e == "$want_executed" ]]; then
        printf '  OK   %s  repeats=%s executed=%s\n' "$probe" "$got_r" "$got_e"
    else
        printf '  FAIL %s  want repeats=%s executed=%s, got: %s\n' \
            "$probe" "$want_repeats" "$want_executed" "${line:-<no CENSUS-INCLUDE line>}"
        rc=1
    fi
}
echo "census calibration ($SIGIL)"
check p1 1 2
check p4 1 4
check p5 2 3
check p7 2 3
echo "SELFCHECK_EXIT=$rc"
exit $rc
