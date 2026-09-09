#!/usr/bin/env bash
# s1-census.sh - re-derive, end to end, what stops Sonic 1 assembling under sigil.
#
# WHY A SCRIPT AND NOT A NUMBER IN A NOTE.
#
# The census counts what sigil's FRONT END REFUSED. It does not count what is
# left to do, and it never could: a run that dies in the front end never reaches
# layout, link or flatten, so every fix uncovers complaints that were previously
# unreachable and the number is expected to RISE before it falls. A figure read
# off a note is therefore stale the moment anything lands, and this repo has
# already steered off a stale one. Run this instead.
#
# It does four things, in order, and refuses rather than guessing at each:
#
#   1. Makes a PRISTINE detached worktree of the corpus at a named revision.
#      The live s1disasm checkout is NOT usable: it carries modified `.nem`
#      blobs and an `s1built.bin` of the wrong size, and it was rejected as a
#      reference once already.
#   2. Runs the corpus's OWN generator half (corpus-prepare.sh), because the
#      generated `*.inc` are gitignored and a run over a bare checkout counts an
#      absent generator's output as the assembler's defects.
#   3. Measures the baseline (corpus-baseline.sh), which prints the class table,
#      the SET diffs in both directions against a previous run, and the
#      unresolved-symbol name sets.
#   4. Runs the DEPTH probe: stubs the remaining refusals in a scratch COPY and
#      measures again, so the report says what is standing behind the count as
#      well as what the count is. The stub tree is never the pristine tree.
#
# It also plants a CANARY for the unresolved-symbol instrument. That instrument
# returns an empty set on a front-end failure, and an emptiness is not a finding
# unless something proves the instrument could have returned non-empty.
#
# Usage:
#   scripts/s1-census.sh --sigil BIN [--work DIR] [--corpus-repo DIR]
#                        [--rev REV] [--compare FILE] [--no-depth]
#
#   --sigil BIN        the sigil binary to measure (required). Build it with
#                      CARGO_TARGET_DIR pointed somewhere that is NOT the shared
#                      checkout's `target/`: relinking `target/release/sigil`
#                      destroys another lane's md5-pinned freeze.
#   --work DIR         scratch root, on DISK (never /tmp, which is tmpfs here).
#   --corpus-repo DIR  the s1disasm git repo to take a worktree FROM.
#   --rev REV          the corpus revision to measure at. Recorded in the report.
#   --compare FILE     a previous run's .err, for the set diff.
#   --no-depth         skip step 4.
#
# Exit status: 0 when a census was reported, nonzero when it could not be.
set -u

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

SIGIL=""
WORK="/home/volence/sonic_hacks/.s1census"
CORPUS_REPO="/home/volence/sonic_hacks/s1disasm"
REV="f6ece657"
COMPARE=""
DEPTH=1
while [ $# -gt 0 ]; do
    case "$1" in
        --sigil) SIGIL="${2:-}"; shift 2 ;;
        --work) WORK="${2:-}"; shift 2 ;;
        --corpus-repo) CORPUS_REPO="${2:-}"; shift 2 ;;
        --rev) REV="${2:-}"; shift 2 ;;
        --compare) COMPARE="${2:-}"; shift 2 ;;
        --no-depth) DEPTH=0; shift ;;
        *) echo "FATAL: unknown argument '$1'" >&2; exit 2 ;;
    esac
done
[ -n "$SIGIL" ] || { sed -n '/^# Usage:/,/^# Exit status/p' "${BASH_SOURCE[0]}" | sed 's/^# \?//' >&2; exit 2; }
[ -x "$SIGIL" ] || { echo "FATAL: '$SIGIL' is not an executable" >&2; exit 2; }
SIGIL="$(cd "$(dirname "$SIGIL")" && pwd)/$(basename "$SIGIL")"
[ -d "$CORPUS_REPO/.git" ] || { echo "FATAL: '$CORPUS_REPO' is not a git checkout" >&2; exit 2; }

case "$WORK" in
    /tmp/*|/dev/shm/*)
        echo "FATAL: --work '$WORK' is on tmpfs. This machine's /tmp is RAM and a" >&2
        echo "       build or a 27 MB corpus copy there wedges the shell." >&2
        exit 2 ;;
esac
mkdir -p "$WORK" || exit 2

PRISTINE="$WORK/s1disasm"
STUB="$WORK/s1disasm-stub"
PROBE="$WORK/probe"

echo "=============================== S1 CENSUS ==============================="
echo "  sigil        $SIGIL"
echo "  sigil md5    $(md5sum "$SIGIL" | cut -d' ' -f1)"
echo "  corpus repo  $CORPUS_REPO"
echo "  corpus rev   $REV"
echo "  work         $WORK"
echo "  started      $(date -u +%FT%TZ)"
echo "  uptime       $(uptime)"
echo

# ---------------------------------------------------------------------------
# 1. Pristine corpus worktree.
# ---------------------------------------------------------------------------
echo "== 1. pristine corpus =="
if [ -d "$PRISTINE" ]; then
    echo "  reusing $PRISTINE"
else
    git -C "$CORPUS_REPO" worktree add --detach "$PRISTINE" "$REV" || exit 3
fi
ACTUAL="$(git -C "$PRISTINE" rev-parse HEAD)"
DIRT="$(git -C "$PRISTINE" status --porcelain | wc -l)"
echo "  HEAD         $ACTUAL"
echo "  dirty        $DIRT path(s)"
if [ "$DIRT" -ne 0 ]; then
    echo "  MODIFIED:"
    git -C "$PRISTINE" status --porcelain | sed 's/^/    /'
    echo "  REFUSED: the corpus must be pristine. The live checkout was rejected"
    echo "           as a reference for exactly this reason; a dirty worktree is"
    echo "           the same defect one directory over."
    exit 3
fi
echo

# ---------------------------------------------------------------------------
# 2. The corpus's own generator half.
# ---------------------------------------------------------------------------
echo "== 2. prepare (the corpus's own generator) =="
bash "$HERE/corpus-prepare.sh" "$PRISTINE" | sed 's/^/  /' || exit 3
echo

# ---------------------------------------------------------------------------
# 3. The baseline.
# ---------------------------------------------------------------------------
echo "== 3. baseline over the pristine tree =="
mkdir -p "$WORK/out"
BASE_ARGS=(--sigil "$SIGIL" --corpus "$PRISTINE" --entry sonic.asm
           --out "$WORK/out" --label "s1-census")
[ -n "$COMPARE" ] && BASE_ARGS+=(--compare "$COMPARE")
bash "$HERE/corpus-baseline.sh" "${BASE_ARGS[@]}" | sed 's/^/  /'
echo

# ---------------------------------------------------------------------------
# 3b. The canary for the unresolved-symbol instrument.
#
# That instrument reports an EMPTY name set whenever the run dies in the front
# end, which is every run this census makes. An empty set is not a finding
# unless something shows the instrument could have returned non-empty, so plant
# one of the class and confirm it is found. The canary proves the PATTERN fires;
# the line count printed beside it proves the INPUT arrived.
# ---------------------------------------------------------------------------
echo "== 3b. canary: could the unresolved-symbol instrument have found anything? =="
mkdir -p "$PROBE"
printf '\tcpu 68000\n\tdc.b NeverDefinedAnywhereCanary\n\tend\n' > "$PROBE/canary.asm"
( cd "$PROBE" && "$SIGIL" canary.asm ) > "$PROBE/canary.out" 2> "$PROBE/canary.err"
echo "  sigil exit   $?"
python3 - "$PROBE/canary.err" <<'PY' | sed 's/^/  /'
import re, sys
pat = re.compile(r'`([^`]+)`')
names, n = set(), 0
for line in open(sys.argv[1], encoding='utf-8', errors='replace'):
    n += 1
    if 'unresolved' in line or 'undefined' in line or 'dangling' in line:
        names.update(pat.findall(line))
print("input reaching the matcher: %d line(s)" % n)
print("names found: %s" % sorted(names))
if n == 0:
    print("CANARY VACUOUS: no input reached the matcher, so this proved nothing.")
elif not names:
    print("CANARY FAILED: the planted symbol was NOT found. The empty sets this")
    print("census reports over the real streams cannot be believed.")
else:
    print("CANARY PASSED: the instrument fires, so an empty set over the real")
    print("streams is a true zero and not a feed failure.")
PY
echo

# ---------------------------------------------------------------------------
# 4. Depth probe.
# ---------------------------------------------------------------------------
if [ "$DEPTH" -eq 1 ]; then
    echo "== 4. depth probe: what is standing behind the count =="
    rm -rf "$STUB"
    cp -a "$PRISTINE" "$STUB" || exit 3
    python3 "$HERE/lib/s1_stub_bc.py" "$STUB" | sed 's/^/  /'
    SRC="$?"
    if [ "$SRC" -ne 0 ]; then
        echo "  REFUSED: the stub did not apply, so anything measured over this tree"
        echo "           would be the UNSTUBBED tree reporting a clean-looking result."
        exit 3
    fi
    echo "  -- proof the mutation landed (diff against the pristine tree) --"
    diff -u "$PRISTINE/sound/_smps2asm_inc.asm" "$STUB/sound/_smps2asm_inc.asm" \
        | sed -n '1,40p' | sed 's/^/    /'
    mkdir -p "$WORK/out-stub"
    bash "$HERE/corpus-baseline.sh" --sigil "$SIGIL" --corpus "$STUB" \
        --entry sonic.asm --out "$WORK/out-stub" --label "s1-census-stubbed" \
        --compare "$WORK/out/s1-census.err" | sed 's/^/  /'
    echo
    echo "  READ THIS RESULT CORRECTLY. The stubbed run is NOT a measurement of"
    echo "  sigil on Sonic 1. It is a measurement of what the front end's refusals"
    echo "  were HIDING. The charset stub is not byte-neutral, so no byte claim"
    echo "  survives it. What it establishes is the STAGE the run reaches once the"
    echo "  front end goes clean, which no complaint count can see."
fi

echo
echo "  finished     $(date -u +%FT%TZ)"
echo "S1-CENSUS-END rc=0"
