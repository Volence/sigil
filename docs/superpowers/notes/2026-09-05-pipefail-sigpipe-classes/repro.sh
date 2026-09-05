#!/usr/bin/env bash
# THE REPRODUCTION FOR THE `pipefail` + SIGPIPE CLASS, AT ONE WRITER SIZE.
#
# CORRECTED 2026-09-05. This header first said the reproduction "has to run under
# load, because run serially the fault cannot appear at all". THAT IS WRONG, and it
# is recorded here rather than quietly swapped so nobody re-derives it. The
# governing variable is the WRITER'S SIZE, not concurrency: `boundary.sh` sweeps it
# with ONE worker and no concurrency anywhere and takes this very construct from
# 0/400 to 400/400 on size alone.
#
# What this file measures is one point on that sweep. Its writer is ~1.4 KB, which
# lands in the narrow RACING BAND where scheduling decides — which is the only
# reason it needs thousands of concurrent runs to show anything. A LARGER WRITER
# FAILS HERE ALMOST EVERY TIME, SERIALLY, ONCE. So do not read a small number out of
# this file as "the class is rare"; read it as "this SIZE is rare".
#
# `grep -q` exits the moment it MATCHES. If the writer still owes output past that
# point, its next write lands on a closed pipe: SIGPIPE, exit 141, and `pipefail`
# hands 141 back as the pipeline's status — so an `if` takes the ELSE branch ON A
# MATCH. If the writer had already emitted everything it will ever emit, it is
# finished and no signal is ever delivered.
#
#   usage: repro.sh [runs-per-arm] [workers]
#
# Both arms answer a membership question whose answer is YES. Every `MISS` is a
# wrong answer. The unfixed arm MUST produce some, or this run is evidence of
# nothing and the script says VACUOUS and exits 2 rather than reading as a pass.
set -uo pipefail

RUNS=${1:-9600}
WORKERS=${2:-$(( $(nproc) * 4 ))}
PER=$(( (RUNS + WORKERS - 1) / WORKERS ))
RUNS=$(( PER * WORKERS ))

WORK=$(mktemp -d "${TMPDIR:-/tmp}/pipefail-repro.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

# The haystack is the founding site's own shape: a list of short names printed by a
# shell builtin into a pipe. The needle is the FIRST entry, so grep matches on line
# one and exits at once — the widest window for the writer to still be writing.
{
    echo 'NEEDLE=tranche5_negative_probes'
    echo 'NAMES=(tranche5_negative_probes'
    for i in $(seq 1 136); do echo "  gate_$i"; done
    echo ')'
} > "$WORK/data.sh"

# ── arm A: the defective construct, one worker's worth of iterations ─────────────
cat > "$WORK/unfixed.sh" <<'ARM'
set -uo pipefail
source "$1"
for ((k = 0; k < $2; k++)); do
    if printf '%s\n' "${NAMES[@]}" | grep -qx "$NEEDLE"; then echo MATCH; else echo "MISS status=$?"; fi
done
ARM

# ── arm B: the fix — the membership question asked with no pipe at all ───────────
cat > "$WORK/fixed.sh" <<'ARM'
set -uo pipefail
source "$1"
for ((k = 0; k < $2; k++)); do
    hit=0
    for n in "${NAMES[@]}"; do [[ $n == "$NEEDLE" ]] && { hit=1; break; }; done
    if (( hit )); then echo MATCH; else echo "MISS status=$?"; fi
done
ARM

arm() {  # arm <name> <script> -> echoes the wrong-answer count on stdout
    # Separate lines on purpose: `local a=$1 b="$a"` expands the whole word list
    # BEFORE any of it is assigned, so `$a` there is unset — and under `set -u`
    # that is a fatal error rather than an empty string.
    local name=$1
    local script=$2
    local w
    local out="$WORK/$name.out"
    : > "$out"
    for ((w = 0; w < WORKERS; w++)); do
        bash "$script" "$WORK/data.sh" "$PER" >> "$out" 2>&1 &
    done
    wait
    local total wrong
    total=$(grep -c . "$out")
    wrong=$(grep -c '^MISS' "$out")
    {
        echo "  $name: $wrong wrong answer(s) of $total"
        if [[ $wrong != 0 ]]; then
            grep '^MISS' "$out" | sort | uniq -c | sed 's/^/      /'
        fi
    } >&2
    printf '%s' "$wrong"
}

echo "runs=$RUNS  workers=$WORKERS  ($PER per worker, $(nproc) cpus)"
echo "UNFIXED — \`printf ... | grep -qx\` under pipefail:"
UNFIXED_WRONG=$(arm unfixed "$WORK/unfixed.sh")
echo "FIXED — the same question, no pipe:"
FIXED_WRONG=$(arm fixed "$WORK/fixed.sh")

echo
if [[ $UNFIXED_WRONG == 0 ]]; then
    echo "VACUOUS: the unfixed arm never gave a wrong answer, so nothing here says"
    echo "anything about the fixed one. The lever that actually moves the rate is the"
    echo "WRITER'S SIZE, not the worker count — this arm's writer is ~1.4 KB, which is"
    echo "in the racing band. Grow the name list (or run boundary.sh) before reaching"
    echo "for more workers."
    exit 2
fi
if [[ $FIXED_WRONG != 0 ]]; then
    echo "THE FIX DOES NOT HOLD: the fixed arm gave $FIXED_WRONG wrong answer(s)."
    exit 1
fi
echo "RED on the unfixed construct ($UNFIXED_WRONG/$RUNS), GREEN on the fixed one (0/$RUNS)."
