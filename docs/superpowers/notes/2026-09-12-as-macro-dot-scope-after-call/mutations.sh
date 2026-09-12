#!/usr/bin/env bash
# Half-fix matrix for AS-MACRO-DOT-SCOPE-AFTER-CALL: remove one piece of the
# COMMITTED fix at a time, show the change landed on disk, run the whole
# sigil-frontend-as suite, restore eval.rs from HEAD, verify it clean.
#
#   SCRATCH=<on-disk dir> mutations.sh <log> [M0 M1 ... M8]
#
# Refuses to start a mutation unless eval.rs is clean against HEAD, so it can
# never overwrite uncommitted work. M0 is the whole file at base 07edc95f.
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../../.." && pwd)"
cd "$ROOT" || exit 9
S="${SCRATCH:?set SCRATCH to an on-disk scratch directory (never /tmp)}"
mkdir -p "$S" || exit 9
LOG="$1"; shift
IDS=("$@")
[ ${#IDS[@]} -eq 0 ] && IDS=(M0 M1 M2 M3 M4 M5 M6 M7 M8)
F=crates/sigil-frontend-as/src/eval.rs
{
echo "pwd=$(pwd) head=$(git rev-parse --short HEAD) branch=$(git branch --show-current) ids=${IDS[*]}"
for m in "${IDS[@]}"; do
    echo "==================== $m"
    if ! git diff --quiet HEAD -- "$F"; then
        echo "REFUSED: $F is not clean against HEAD before $m"; exit 3
    fi
    if [ "$m" = M0 ]; then
        git show 07edc95f:"$F" > "$F" || exit 4
        echo "APPLIED M0 (eval.rs at base 07edc95f)"
    else
        python3 "$HERE/mutate.py" "$m" || exit 4
    fi
    echo "-- diff --stat"
    git diff --stat -- "$F"
    echo "-- changed lines on disk"
    git diff -U0 -- "$F" | grep -E '^[-+][^-+]' | head -12
    cargo test --release -p sigil-frontend-as --no-fail-fast > "$S/mut-$m.log" 2>&1
    rc=$?
    echo "-- cargo_exit=$rc"
    echo "-- result lines=$(grep -c '^test result:' "$S/mut-$m.log") running=$(grep -c '^     Running ' "$S/mut-$m.log") doctests=$(grep -c '^   Doc-tests ' "$S/mut-$m.log")"
    grep '^test result:' "$S/mut-$m.log" | awk '{p+=$4; f+=$6; i+=$8} END {print "-- passed", p, "failed", f, "ignored", i}'
    echo "-- failing tests:"
    grep -E '^test .* FAILED$' "$S/mut-$m.log" | sed 's/^/   /'
    git checkout HEAD -- "$F"
    if git diff --quiet HEAD -- "$F"; then echo "-- restored clean"; else echo "RESTORE FAILED"; exit 5; fi
done
echo "MUTATIONS_END_MARKER"
} > "$LOG" 2>&1
