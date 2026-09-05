#!/usr/bin/env bash
# RED-FIRST PROOF FOR THE CI "report reference-dependent skips" STEP.
#
# It runs the step's own `run:` block — extracted from the workflow file rather
# than retyped, so a retyped copy cannot drift away from what CI executes —
# against two beds, on BOTH the committed baseline and the working tree.
#
#   bed A: test-output.txt with NO skip lines. The case the step exists to shout
#          about. On the baseline, `grep '^skip: '` matches nothing, exits 1, and
#          `set -e` kills the step BEFORE the five lines of explanation — the
#          operator gets a bare red. It must print the ERROR text and exit 1.
#   bed B: test-output.txt with far more distinct skip lines than fit in a pipe
#          buffer. On the baseline `head -40` exits, `sort -rn` takes SIGPIPE,
#          `pipefail` returns 141 and `set -e` fails the step — a red arriving
#          only because there was MORE to report. It must exit 0.
#
#   usage: ci_report_step_proof.sh [<repo root>]
#
# The baseline arm MUST fail on both beds. If it does not, this proof measured
# nothing and says so rather than reading as a pass.
set -uo pipefail

ROOT=${1:-$(cd "$(dirname "$0")/../../../.." && pwd)}
YML=$ROOT/.github/workflows/ci.yml
[[ -r $YML ]] || { echo "COULD NOT MEASURE: no workflow at $YML"; exit 2; }

WORK=$(mktemp -d "${TMPDIR:-/tmp}/ci-report-proof.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

# ── the step's shell, taken out of the YAML ─────────────────────────────────────
extract() {  # extract <yaml> <out.sh>
    python3 - "$1" "$2" <<'PY'
import sys
path, out = sys.argv[1], sys.argv[2]
lines = open(path).read().split('\n')
start = None
for i, l in enumerate(lines):
    if 'name: report reference-dependent skips' in l:
        start = i
if start is None:
    raise SystemExit("COULD NOT MEASURE: the step is not in " + path)
j = start
while 'run: |' not in lines[j]:
    j += 1
body, k, indent = [], j + 1, None
while k < len(lines):
    l = lines[k]
    if l.strip() == '':
        body.append('')
        k += 1
        continue
    ind = len(l) - len(l.lstrip())
    if indent is None:
        indent = ind
    if ind < indent:
        break
    body.append(l[indent:])
    k += 1
open(out, 'w').write('\n'.join(body) + '\n')
PY
}

git -C "$ROOT" show HEAD:.github/workflows/ci.yml > "$WORK/base.yml" \
    || { echo "COULD NOT MEASURE: no committed baseline for the workflow"; exit 2; }
extract "$WORK/base.yml" "$WORK/base.sh" || exit 2
extract "$YML"           "$WORK/tree.sh" || exit 2

# The baseline arm has to actually BE the defective construct, or a "red" from it
# proves nothing about this class.
grep -q 'head -40$' "$WORK/base.sh" || {
    echo "COULD NOT MEASURE: the committed baseline's pipeline does not end in \`head -40\`;"
    echo "the defect this proof is pointed at is not in the file it is reading."
    exit 2
}
grep -q 'head -40$' "$WORK/tree.sh" && {
    echo "THE WORKING TREE STILL ENDS THE PIPELINE IN \`head -40\` — nothing was fixed."
    exit 1
}

# ── the beds ────────────────────────────────────────────────────────────────────
mkdir -p "$WORK/bedA" "$WORK/bedB"
printf 'test result: ok. 10 passed; 0 failed\n' > "$WORK/bedA/test-output.txt"
# Enough distinct lines that `sort -rn`'s output is far larger than a pipe buffer,
# so `head -40` leaving early is a certainty rather than a race.
python3 -c "
import sys
w = open(sys.argv[1], 'w')
for i in range(100000):
    w.write('skip: gate_%d needs a reference tree; set AEON_DIR to /some/where/aeon-%d\n' % (i, i))
" "$WORK/bedB/test-output.txt"

run() {  # run <script> <bed> -> prints "<exit> <lines of output>"; output to $WORK/last
    ( cd "$WORK/$2" && bash "$WORK/$1" ) > "$WORK/last" 2>&1
    echo "$?"
}

verdict=0
for bed in bedA bedB; do
    echo "================ $bed"
    for arm in base tree; do
        rc=$(run "$arm.sh" "$bed")
        err_text=no
        grep -q '^ERROR: expected reference-dependent skips' "$WORK/last" && err_text=yes
        trunc=no
        grep -q 'further distinct skip line' "$WORK/last" && trunc=yes
        printf '  %-5s exit=%-4s explains-itself=%-4s says-it-truncated=%-4s output_lines=%s\n' \
            "$arm" "$rc" "$err_text" "$trunc" "$(grep -c . "$WORK/last")"
        [[ $arm == base ]] && printf '        base tail: %s\n' "$(tail -1 "$WORK/last")"
    done

    if [[ $bed == bedA ]]; then
        rc_base=$(run base.sh bedA); base_says=$(grep -c '^ERROR: expected' "$WORK/last")
        rc_tree=$(run tree.sh bedA); tree_says=$(grep -c '^ERROR: expected' "$WORK/last")
        # RED REQUIRED: the baseline dies without the explanation.
        if [[ $rc_base == 0 || $base_says != 0 ]]; then
            echo "  VACUOUS: the baseline explained itself on an empty bed — the defect is not here."
            verdict=2
        elif [[ $rc_tree != 1 || $tree_says == 0 ]]; then
            echo "  THE FIX DOES NOT HOLD: the tree must exit 1 AND print the ERROR text."
            verdict=1
        else
            echo "  OK: baseline exit=$rc_base with no explanation; tree exit=$rc_tree WITH it."
        fi
    else
        rc_base=$(run base.sh bedB)
        rc_tree=$(run tree.sh bedB); tree_trunc=$(grep -c 'further distinct skip line' "$WORK/last")
        if [[ $rc_base == 0 ]]; then
            echo "  VACUOUS: the baseline survived a 100k-line bed — no SIGPIPE was taken,"
            echo "  so this bed says nothing about the fix."
            verdict=2
        elif [[ $rc_tree != 0 || $tree_trunc == 0 ]]; then
            echo "  THE FIX DOES NOT HOLD: the tree must exit 0 and say that it truncated."
            verdict=1
        else
            echo "  OK: baseline exit=$rc_base (SIGPIPE, 141); tree exit=$rc_tree and names the tail it dropped."
        fi
    fi
done
exit "$verdict"
