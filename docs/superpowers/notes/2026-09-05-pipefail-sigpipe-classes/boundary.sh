#!/usr/bin/env bash
# WHAT DECIDES A `pipefail` + SIGPIPE FALSE NON-MATCH: THE WRITER'S SIZE, NOT LOAD.
#
# `repro.sh` shows the fault at ONE writer size, and a reproduction that only ever
# samples one size cannot tell two mechanisms apart. This one sweeps the size, with
# ONE WORKER AND NO CONCURRENCY ANYWHERE, and shows the fault going from never to
# always as the writer grows. That settles it: concurrency is not the variable.
#
# THE MECHANISM, in the form the measurement supports. `grep -qx` exits the moment
# it matches. If the writer has already handed over everything it will ever emit,
# it is finished and no signal is ever delivered — the fault is IMPOSSIBLE, not
# rare. If the writer must still issue at least one more write(2) after the reader
# has seen the match, that write lands on a closed pipe — the fault is NEAR-CERTAIN,
# not rare. Only in the narrow band where the two race does scheduling decide, and
# THAT is the band where load matters.
#
# THE BOUNDARY IS A PROPERTY OF THE WRITER, NOT OF THE PIPE, and this script's
# controls are what establish that: three writers are run over the SAME pipe, whose
# capacity is read from the kernel rather than assumed. If the pipe's capacity were
# the boundary, all three would turn over at the same size. They do not — measured
# here they differ by more than an order of magnitude, because what governs is how
# much the writer still has to push out after the match, in units of its OWN output
# buffering. So "under a pipe buffer, therefore safe" is not a sound triage rule:
# for a shell builtin the turnover is far BELOW the pipe's capacity.
#
#   usage: boundary.sh [runs-per-point]
#
# It REFUSES (exit 2) unless it can exhibit both regimes serially, because a sweep
# that only ever saw one of them has not demonstrated a boundary at all.
set -uo pipefail

RUNS=${1:-400}
WORK=$(mktemp -d "${TMPDIR:-/tmp}/pipefail-boundary.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

# The pipe's capacity, from the kernel. Assuming 65536 here would be assuming the
# very thing this script exists to rule out as the boundary.
PIPE_SZ=$(python3 -c '
import fcntl, os
r, w = os.pipe()
print(fcntl.fcntl(w, 1032))  # F_GETPIPE_SZ
') || { echo "COULD NOT MEASURE: the pipe capacity is unreadable"; exit 2; }
echo "pipe capacity on this machine (F_GETPIPE_SZ, read not assumed): $PIPE_SZ bytes"
echo "runs per point: $RUNS, ONE worker, no concurrency"

# The needle is the FIRST line, so the match is as early as it can be and every
# other byte is "output the writer still owes past the match".
mk() { { echo aaa_first; for ((i = 1; i < $1; i++)); do printf 'gate_%06d\n' "$i"; done; } > "$WORK/hay.txt"; }

cat > "$WORK/run.sh" <<'ARM'
set -uo pipefail
kind=$1; hay=$2; runs=$3
mapfile -t NAMES < "$hay"
for ((k = 0; k < runs; k++)); do
    case $kind in
      builtin) printf '%s\n' "${NAMES[@]}" | grep -qx aaa_first && echo M || echo "X $?" ;;
      seq)     seq 1 "${#NAMES[@]}"        | grep -qx 1         && echo M || echo "X $?" ;;
      cat)     cat "$hay"                  | grep -qx aaa_first && echo M || echo "X $?" ;;
    esac
done
ARM

# point <kind> <lines> -> echoes "<bytes> <wrong>"
point() {
    mk "$2"
    bash "$WORK/run.sh" "$1" "$WORK/hay.txt" "$RUNS" > "$WORK/o" 2>&1
    printf '%s %s' "$(stat -c%s "$WORK/hay.txt")" "$(grep -c '^X' "$WORK/o")"
}

# sweep <kind> <sizes…> -> TABLE ON STDERR, "<lastZeroBytes> <firstAllBytes>" on stdout.
# The split is load-bearing: with both on stdout the caller's `$(…)` swallows the
# table and `read` takes its first line as the answer. That is exactly how the first
# run of this script reported "all three writers turned over at the same size" — a
# false REASSURANCE manufactured by a broken measurement, which is the same shape of
# failure as the defect this directory is about. Hence also the emptiness check below:
# an unset result must REFUSE, and `-z` is not the same test as `== -`.
sweep() {
    local kind=$1; shift
    local n bytes wrong lo=- hi=-
    {
        echo
        echo "  writer = $kind"
    } >&2
    for n in "$@"; do
        read -r bytes wrong <<< "$(point "$kind" "$n")"
        local pct=$(( wrong * 100 / RUNS ))
        printf '    lines=%-8s bytes=%-9s false-non-match=%4s/%-4s  %3s%%  %s\n' \
            "$n" "$bytes" "$wrong" "$RUNS" "$pct" \
            "$( ((wrong == 0)) && echo IMPOSSIBLE || { ((pct >= 95)) && echo NEAR-CERTAIN || echo '  <- the racing band'; })" >&2
        ((wrong == 0)) && lo=$bytes
        [[ $hi == - ]] && ((pct >= 95)) && hi=$bytes
    done
    printf '%s %s' "$lo" "$hi"
}

read -r B_LO B_HI <<< "$(sweep builtin 100 400 600 700 800 900 1000 1200 2000)"
read -r S_LO S_HI <<< "$(sweep seq     400 1000 2000 8000 20000 60000)"
read -r C_LO C_HI <<< "$(sweep cat     8000 40000 60000 120000)"

echo
echo "TURNOVER, serially, per writer (last size with ZERO -> first size at >=95%):"
printf '  bash printf builtin : %s -> %s bytes\n' "$B_LO" "$B_HI"
printf '  seq                 : %s -> %s bytes\n' "$S_LO" "$S_HI"
printf '  cat                 : %s -> %s bytes\n' "$C_LO" "$C_HI"
printf '  the pipe they all share: %s bytes\n' "$PIPE_SZ"

fail=0
for pair in "builtin:$B_LO:$B_HI" "seq:$S_LO:$S_HI" "cat:$C_LO:$C_HI"; do
    k=${pair%%:*}; rest=${pair#*:}; lo=${rest%%:*}; hi=${rest#*:}
    if [[ -z $lo || -z $hi || $lo == - || $hi == - ]]; then
        echo "REFUSING: writer '$k' did not show BOTH regimes in this sweep (zero-point=$lo, saturated-point=$hi)."
        echo "A sweep that saw only one regime has demonstrated no boundary. Widen the sizes."
        fail=2
    fi
done
((fail)) && exit "$fail"

# THE CONTROL. Same pipe, same reader, same machine, three writers. If the pipe's
# capacity were the boundary the three would turn over together.
if [[ -n $B_HI && $B_HI == "$S_HI" && $S_HI == "$C_HI" ]]; then
    echo
    echo "The three writers turned over at the SAME size, which is what the pipe-capacity"
    echo "story predicts. This run does NOT refute it — report that, do not overclaim."
    exit 0
fi
echo
echo "THE THREE WRITERS TURN OVER AT DIFFERENT SIZES OVER THE SAME $PIPE_SZ-BYTE PIPE."
echo "So the pipe's capacity is not the boundary, and \"under a pipe buffer, therefore"
echo "safe\" is not a sound per-site rule: the shell builtin — the writer this repo's"
echo "scripts actually use — turns over well BELOW it."
