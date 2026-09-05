#!/usr/bin/env bash
# RED-FIRST PROOF FOR `docs/superpowers/notes/asl-reference/selfcheck.sh` CASE 3.
#
# Case 3 asks whether the guard's refusal message names both digests. The answer
# on a correct message is YES. The test was
#
#     if printf '%s' "$msg" | grep -q "$A" && printf '%s' "$msg" | grep -q "$B"; then
#
# under that file's own `set -o pipefail`, which reads a match as a NON-match
# whenever `grep -q` exits before `printf`'s write lands — a false FAIL of a case
# that passed.
#
# Both forms are lifted OUT OF THE TWO VERSIONS OF THE FILE rather than retyped,
# so this cannot end up proving something about a construct the file does not
# contain. Both are then run under load against a message that DOES carry both
# digests; every "MISS" is a wrong answer.
#
#   usage: selfcheck_case3_proof.sh [runs] [workers]
#
# The baseline arm MUST produce wrong answers. A zero there is a VACUOUS run.
set -uo pipefail

RUNS=${1:-9600}
WORKERS=${2:-$(( $(nproc) * 4 ))}
PER=$(( (RUNS + WORKERS - 1) / WORKERS ))
RUNS=$(( PER * WORKERS ))

ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
REL=docs/superpowers/notes/asl-reference/selfcheck.sh
WORK=$(mktemp -d "${TMPDIR:-/tmp}/case3-proof.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

git -C "$ROOT" show "HEAD:$REL" > "$WORK/base.sh" \
    || { echo "COULD NOT MEASURE: no committed baseline for $REL"; exit 2; }

# The decision line is the one immediately above case 3's PASS call, in both
# versions — an anchor in the file, not a line number.
decision() {  # decision <file>
    awk '/ok "message carries both the wanted and the seen digest"/ { print prev; exit } { prev = $0 }' "$1"
}
BASE_LINE=$(decision "$WORK/base.sh")
TREE_LINE=$(decision "$ROOT/$REL")
[[ -n $BASE_LINE && -n $TREE_LINE ]] || {
    echo "COULD NOT MEASURE: case 3's decision line was not found in one of the two versions"
    exit 2
}
echo "baseline decision: $BASE_LINE"
echo "tree decision:     $TREE_LINE"
[[ $BASE_LINE == *'grep -q'* ]] || {
    echo "COULD NOT MEASURE: the baseline's decision does not pipe into \`grep -q\`, so the"
    echo "defect this proof is pointed at is not in the file it read."
    exit 2
}
[[ $TREE_LINE == *'|'* ]] && {
    echo "THE WORKING TREE STILL PIPES — nothing was fixed."
    exit 1
}

# A message shaped like the guard's real refusal, carrying both digests.
cat > "$WORK/data.sh" <<'DATA'
VARYING_MD5=0dee1f98e6480a4783d27ffd8b90896f
STABLE_MD5=61e672562465725a8c102288a7da9098
msg="asl_ref.sh: REFUSED — the asl on this path is md5 $VARYING_MD5, and this lane
pins the reference build at md5 $STABLE_MD5. The two print an identical banner,
which is why the digest and not the version is the check."
DATA

arm() {  # arm <name> <decision line> -> echoes the wrong-answer count
    local name=$1
    local line=$2
    local script="$WORK/$name.sh"
    local out="$WORK/$name.out"
    local w
    {
        echo 'set -uo pipefail'
        echo "source \"$WORK/data.sh\""
        echo 'for ((k = 0; k < $1; k++)); do'
        echo "    $line"
        echo '        echo MATCH'
        echo '    else'
        echo '        echo "MISS status=$?"'
        echo '    fi'
        echo 'done'
    } > "$script"
    bash -n "$script" || { echo "COULD NOT MEASURE: $name arm does not parse" >&2; return 2; }
    : > "$out"
    for ((w = 0; w < WORKERS; w++)); do
        bash "$script" "$PER" >> "$out" 2>&1 &
    done
    wait
    {
        echo "  $name: $(grep -c '^MISS' "$out") wrong answer(s) of $(grep -c . "$out")"
        grep '^MISS' "$out" | sort | uniq -c | sed 's/^/      /'
    } >&2
    printf '%s' "$(grep -c '^MISS' "$out")"
}

echo "runs=$RUNS workers=$WORKERS ($PER per worker, $(nproc) cpus)"
echo "BASELINE construct:"
BASE_WRONG=$(arm base "$BASE_LINE")
echo "TREE construct:"
TREE_WRONG=$(arm tree "$TREE_LINE")

echo
if [[ $BASE_WRONG == 0 ]]; then
    echo "VACUOUS: the baseline construct never gave a wrong answer here, so this run"
    echo "says nothing about the fixed one. Raise the worker or run count."
    exit 2
fi
if [[ $TREE_WRONG != 0 ]]; then
    echo "THE FIX DOES NOT HOLD: the tree construct gave $TREE_WRONG wrong answer(s)."
    exit 1
fi
echo "RED on the committed construct ($BASE_WRONG/$RUNS), GREEN on the tree's ($TREE_WRONG/$RUNS)."
