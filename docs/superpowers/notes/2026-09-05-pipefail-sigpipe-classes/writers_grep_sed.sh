#!/usr/bin/env bash
# THE TURNOVER FOR THE WRITERS THIS REPO ACTUALLY PIPES INTO `head`.
#
# `boundary.sh` establishes that the turnover is a property of the WRITER and not of
# the pipe, using three writers — a bash `printf` builtin, `seq`, and `cat`. None of
# those is what this tree writes. Every one of the ten `$( … | head -N )` sites feeds
# `head` from a `grep` or a `sed` or a shell builtin, and the note that priced them
# said in as many words that `sort`'s turnover "was never measured here". Pricing a
# site by a turnover measured on a different program is the same move as pricing it by
# the pipe's capacity: it borrows a number from something that is not the subject.
#
# So this measures `grep` and `sed` directly, in the SHAPE THE SITES USE — a reader of
# `head -N` rather than `grep -q`, because `head -N` exits after N lines and N is not
# 1 at the site that matters (`head -20`).
#
#   usage: writers_grep_sed.sh [runs-per-point] [head-N]
#
# It REFUSES (exit 2) unless every writer exhibits BOTH regimes, for boundary.sh's
# reason: a sweep that saw only one regime has demonstrated no boundary.
set -uo pipefail

RUNS=${1:-200}
HEADN=${2:-20}
WORK=$(mktemp -d "${TMPDIR:-/tmp}/pipefail-writers.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

PIPE_SZ=$(python3 -c '
import fcntl, os
r, w = os.pipe()
print(fcntl.fcntl(w, 1032))  # F_GETPIPE_SZ
') || { echo "COULD NOT MEASURE: the pipe capacity is unreadable"; exit 2; }

echo "pipe capacity on this machine (F_GETPIPE_SZ, read not assumed): $PIPE_SZ bytes"
echo "runs per point: $RUNS, ONE worker, no concurrency; reader is \`head -$HEADN\`"
echo "every line matches, so the reader has its N lines immediately and everything"
echo "past them is output the writer still owes."

mk() { for ((i = 0; i < $1; i++)); do printf 'row_%08d some payload text\n' "$i"; done > "$WORK/hay.txt"; }

cat > "$WORK/run.sh" <<'ARM'
set -uo pipefail
kind=$1; hay=$2; runs=$3; n=$4
mapfile -t ROWS < "$hay"
for ((k = 0; k < runs; k++)); do
    case $kind in
      grep)    grep -v '^never_matches_anything' "$hay" | head -"$n" >/dev/null ;;
      sed)     sed -n 'p'                        "$hay" | head -"$n" >/dev/null ;;
      awk)     awk '{print}'                     "$hay" | head -"$n" >/dev/null ;;
      builtin) printf '%s\n' "${ROWS[@]}"               | head -"$n" >/dev/null ;;
    esac
    rc=$?
    ((rc == 0)) || echo "X $rc"
done
ARM

point() {
    mk "$2"
    bash "$WORK/run.sh" "$1" "$WORK/hay.txt" "$RUNS" "$HEADN" > "$WORK/o" 2>&1
    printf '%s %s' "$(stat -c%s "$WORK/hay.txt")" "$(grep -c '^X' "$WORK/o")"
}

# Table to stderr, "<lastZeroBytes> <firstSaturatedBytes>" to stdout — boundary.sh's
# split, for boundary.sh's reason: with both on stdout the caller's `$(…)` swallows
# the table and `read` takes the table's first line as the answer.
sweep() {
    local kind=$1; shift
    local n bytes wrong lo=- hi=-
    { echo; echo "  writer = $kind"; } >&2
    for n in "$@"; do
        read -r bytes wrong <<< "$(point "$kind" "$n")"
        local pct=$(( wrong * 100 / RUNS ))
        printf '    lines=%-9s bytes=%-10s sigpipe=%4s/%-4s  %3s%%  %s\n' \
            "$n" "$bytes" "$wrong" "$RUNS" "$pct" \
            "$( ((wrong == 0)) && echo IMPOSSIBLE || { ((pct >= 95)) && echo NEAR-CERTAIN || echo '  <- the racing band'; })" >&2
        ((wrong == 0)) && lo=$bytes
        [[ $hi == - ]] && ((pct >= 95)) && hi=$bytes
    done
    printf '%s %s' "$lo" "$hi"
}

read -r G_LO G_HI <<< "$(sweep grep    $((HEADN + 1)) 100 400 1000 2000 4000 20000 200000)"
read -r S_LO S_HI <<< "$(sweep sed     $((HEADN + 1)) 100 400 1000 2000 4000 20000 200000)"
read -r A_LO A_HI <<< "$(sweep awk     $((HEADN + 1)) 100 400 1000 2000 4000 20000 200000)"
read -r B_LO B_HI <<< "$(sweep builtin $((HEADN + 1)) 100 200 300 400 600 1000 4000)"

echo
echo "TURNOVER, serially, per writer (last size with ZERO -> first size at >=95%):"
printf '  grep -v             : %s -> %s bytes\n' "$G_LO" "$G_HI"
printf '  sed -n p            : %s -> %s bytes\n' "$S_LO" "$S_HI"
printf '  awk {print}         : %s -> %s bytes\n' "$A_LO" "$A_HI"
printf '  bash printf builtin : %s -> %s bytes\n' "$B_LO" "$B_HI"
printf '  the pipe they all share: %s bytes\n' "$PIPE_SZ"

fail=0
for pair in "grep:$G_LO:$G_HI" "sed:$S_LO:$S_HI" "awk:$A_LO:$A_HI" "builtin:$B_LO:$B_HI"; do
    k=${pair%%:*}; rest=${pair#*:}; lo=${rest%%:*}; hi=${rest#*:}
    if [[ -z $lo || -z $hi || $lo == - || $hi == - ]]; then
        echo "REFUSING: writer '$k' did not show BOTH regimes in this sweep (zero-point=$lo, saturated-point=$hi)."
        echo "A sweep that saw only one regime has demonstrated no boundary. Widen the sizes."
        fail=2
    fi
done
((fail)) && exit "$fail"
exit 0
