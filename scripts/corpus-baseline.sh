#!/usr/bin/env bash
# corpus-baseline.sh - the SIGIL-AS-REPLACEMENT corpus diagnostic baseline.
#
# Runs one sigil binary over one community disassembly and reports the
# diagnostic count decomposed by class, with the provenance of every number.
#
# WHY THIS SCRIPT EXISTS AT ALL.
#
# The corpus disassemblies have build-time generated include files
# (`sound/DAC/generated/*.inc` in s2disasm, `sound/dac/dpcm/generated/*.inc` in
# s1disasm) that their own `build.lua` writes from `.wav` and `.asm` sources. A
# bare `git` checkout has none of them, and they are gitignored, so nothing in a
# fresh clone hints that they are absent. A run over a bare checkout counts the
# assembler's defects PLUS an absent generator's output and cannot tell them
# apart: on s2disasm at e45ebf33 that is 39 `cannot include` rows plus a
# downstream shadow of 17 more, all of them the corpus's build system missing
# rather than the assembler's fault. Every count this project has steered by was
# measured that way.
#
# So this script REFUSES to report a baseline over a tree that is missing its
# generated includes. The expectation is DERIVED, not declared: the corpus's own
# assembly sources name the paths, and the check is that every named path is on
# disk. There is no maintained list to go stale, and the population it checked
# is printed beside the verdict so a green over an empty population is visible
# as such rather than reading as a pass.
#
# Usage:
#   scripts/corpus-baseline.sh --sigil BIN --corpus DIR --entry FILE [options]
#
#   --sigil BIN        the sigil binary to measure (required)
#   --corpus DIR       the corpus tree, prepared by scripts/corpus-prepare.sh
#   --entry FILE       the root assembly file, relative to the corpus dir
#   --out DIR          where to write the diagnostic streams (default: a temp dir)
#   --label NAME       a name for this run, used in filenames and the verdict
#   --compare FILE     a previous run's .err to diff this one against
#   --unprepared-ok    measure anyway over a tree missing generated includes,
#                      and stamp the report UNPREPARED. The number it prints is
#                      NOT a baseline and the verdict says so.
#
# Exit status: 0 on a reported baseline, 3 when the tree is unprepared, 4 when a
# count could not be measured at all, 2 on usage. "Could not measure" is never
# rendered as 0 and never as green.
set -u

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLASSES="$HERE/lib/corpus_classes.py"

SIGIL=""; CORPUS=""; ENTRY=""; OUT=""; LABEL=""; COMPARE=""; UNPREPARED_OK=0
while [ $# -gt 0 ]; do
    case "$1" in
        --sigil) SIGIL="${2:-}"; shift 2 ;;
        --corpus) CORPUS="${2:-}"; shift 2 ;;
        --entry) ENTRY="${2:-}"; shift 2 ;;
        --out) OUT="${2:-}"; shift 2 ;;
        --label) LABEL="${2:-}"; shift 2 ;;
        --compare) COMPARE="${2:-}"; shift 2 ;;
        --unprepared-ok) UNPREPARED_OK=1; shift ;;
        *) echo "FATAL: unknown argument '$1'" >&2; exit 2 ;;
    esac
done
[ -n "$SIGIL" ] && [ -n "$CORPUS" ] && [ -n "$ENTRY" ] || {
    sed -n '/^# Usage:/,/^# Exit status/p' "${BASH_SOURCE[0]}" | sed 's/^# \?//' >&2
    exit 2
}
[ -x "$SIGIL" ] || { echo "FATAL: '$SIGIL' is not an executable" >&2; exit 2; }
[ -d "$CORPUS" ] || { echo "FATAL: '$CORPUS' is not a directory" >&2; exit 2; }
SIGIL="$(cd "$(dirname "$SIGIL")" && pwd)/$(basename "$SIGIL")"
CORPUS="$(cd "$CORPUS" && pwd)"
[ -f "$CORPUS/$ENTRY" ] || { echo "FATAL: '$CORPUS/$ENTRY' does not exist" >&2; exit 2; }
[ -f "$CLASSES" ] || { echo "FATAL: missing $CLASSES" >&2; exit 2; }
[ -z "$LABEL" ] && LABEL="$(basename "$CORPUS")"
if [ -z "$OUT" ]; then OUT="$(mktemp -d)"; fi
mkdir -p "$OUT" || exit 2

ERR="$OUT/$LABEL.err"
STDOUT="$OUT/$LABEL.out"

# ---------------------------------------------------------------------------
# Provenance. Every number below is only as good as this block.
# ---------------------------------------------------------------------------
echo "== provenance =="
echo "  label       $LABEL"
echo "  sigil       $SIGIL"
echo "  sigil md5   $(md5sum "$SIGIL" | cut -d' ' -f1)  size $(stat -c%s "$SIGIL") bytes"
echo "  sigil built $(stat -c%y "$SIGIL" | cut -d. -f1)"
echo "  corpus      $CORPUS"
echo "  corpus rev  $(cd "$CORPUS" && git rev-parse HEAD 2>/dev/null || echo '(not a git tree)')"
echo "  corpus dirty $(cd "$CORPUS" && git status --porcelain 2>/dev/null | wc -l) path(s)"
echo "  entry       $ENTRY"
echo "  started     $(date -u +%FT%TZ)"

# ---------------------------------------------------------------------------
# Readiness, derived from the corpus's own assembly sources.
#
# The population is every distinct double-quoted path naming a `generated/`
# component anywhere in the corpus's tracked `.asm` and `.inc` files. That is
# the set of build-time artifacts the assembler will be asked for. Nothing here
# is written down in this repository, so nothing here can go stale when a corpus
# adds a generated include.
# ---------------------------------------------------------------------------
echo
echo "== generated-include readiness (derived from the corpus's own sources) =="
WANTED="$OUT/$LABEL.generated-wanted.txt"
MISSING="$OUT/$LABEL.generated-missing.txt"
if ( cd "$CORPUS" && git rev-parse --git-dir >/dev/null 2>&1 ); then
    ( cd "$CORPUS" && git grep -hoE '"[^"]*generated/[^"]*"' -- '*.asm' '*.inc' ) \
        | tr -d '"' | sort -u > "$WANTED"
    SCOPE="tracked *.asm and *.inc"
else
    echo "  REFUSED: '$CORPUS' is not a git tree, so the corpus's own tracked sources"
    echo "           cannot be enumerated and readiness cannot be derived."
    echo "  VERDICT: UNMEASURABLE"
    exit 4
fi
NWANT=$(wc -l < "$WANTED")
: > "$MISSING"
while IFS= read -r p; do
    [ -z "$p" ] && continue
    [ -e "$CORPUS/$p" ] || printf '%s\n' "$p" >> "$MISSING"
done < "$WANTED"
NMISS=$(wc -l < "$MISSING")
echo "  scope       $SCOPE"
echo "  population  $NWANT generated path(s) named by the corpus"
echo "  present     $((NWANT - NMISS))"
echo "  missing     $NMISS"

READY_STATE="READY"
if [ "$NWANT" -eq 0 ]; then
    # Not a pass. The corpus names no generated includes, so this check proved
    # nothing about this tree and says so rather than printing a reassuring 0.
    READY_STATE="VACUOUS"
    echo "  NOTE: the population is EMPTY. This check asked the corpus which"
    echo "        generated files it needs and the corpus named none, so it has"
    echo "        NOT established that the tree is prepared. The post-run check"
    echo "        below is the only readiness evidence this run carries."
elif [ "$NMISS" -gt 0 ]; then
    READY_STATE="UNPREPARED"
    echo "  MISSING PATHS (first 20):"
    head -20 "$MISSING" | sed 's/^/    /'
    echo
    echo "  This tree has not had its generator run. Prepare it with:"
    echo "      scripts/corpus-prepare.sh $CORPUS"
    if [ "$UNPREPARED_OK" -ne 1 ]; then
        echo
        echo "  VERDICT: REFUSED. A count over this tree measures the assembler's"
        echo "           defects PLUS $NMISS absent generated file(s) and cannot"
        echo "           separate them. Pass --unprepared-ok to measure anyway;"
        echo "           the result is not a baseline."
        echo "CORPUS-BASELINE-END rc=3"
        exit 3
    fi
    echo "  --unprepared-ok given: measuring anyway, stamped UNPREPARED."
fi

# ---------------------------------------------------------------------------
# The run.
# ---------------------------------------------------------------------------
echo
echo "== run =="
( cd "$CORPUS" && "$SIGIL" "$ENTRY" ) > "$STDOUT" 2> "$ERR"
RC=$?
NERR=$(wc -l < "$ERR")
NOUT=$(wc -l < "$STDOUT")
echo "  sigil exit  $RC"
echo "  stderr      $NERR line(s)  -> $ERR"
echo "  stdout      $NOUT line(s)  -> $STDOUT"
echo "  finished    $(date -u +%FT%TZ)"

if [ "$RC" -eq 0 ] && [ "$NERR" -eq 0 ]; then
    echo "  NOTE: exit 0 with an empty stream. That is a clean assembly, and the"
    echo "        class table below will say so over a population of 0."
fi
# A crash leaves a truncated stream that looks exactly like a low count.
if [ "$RC" -gt 1 ]; then
    echo
    echo "  VERDICT: UNMEASURABLE. sigil exited $RC, which is neither a clean run"
    echo "           nor a diagnostic run, so the $NERR line(s) it managed to emit"
    echo "           are a truncated stream and not a count. Last 5 lines:"
    tail -5 "$ERR" | sed 's/^/    /'
    echo "CORPUS-BASELINE-END rc=4"
    exit 4
fi

# ---------------------------------------------------------------------------
# Post-run readiness: the run's own evidence, independent of the source scan.
#
# The source scan can only see paths spelled as literals. This one sees whatever
# the assembler actually failed to find, whether or not this script could parse
# the way it was spelled.
# ---------------------------------------------------------------------------
echo
echo "== generated-include readiness (derived from the run itself) =="
NGENDIAG=$(grep -c 'generated/' "$ERR")
echo "  diagnostics naming a generated/ path: $NGENDIAG  (of $NERR scanned)"
if [ "$NGENDIAG" -gt 0 ]; then
    grep 'generated/' "$ERR" | head -5 | sed 's/^/    /'
    READY_STATE="UNPREPARED"
    if [ "$UNPREPARED_OK" -ne 1 ]; then
        echo
        echo "  VERDICT: REFUSED. The run itself complained about $NGENDIAG"
        echo "           generated path(s), so this tree is not prepared however"
        echo "           the source scan read."
        echo "CORPUS-BASELINE-END rc=3"
        exit 3
    fi
fi

# ---------------------------------------------------------------------------
# The count, decomposed.
# ---------------------------------------------------------------------------
echo
echo "== class table =="
python3 "$CLASSES" "$ERR"
CRC=$?

if [ -n "$COMPARE" ]; then
    if [ ! -f "$COMPARE" ]; then
        echo
        echo "  NOTE: --compare '$COMPARE' does not exist; no comparison was made."
        echo "        This is reported rather than skipped silently."
    else
        echo
        echo "== against $COMPARE =="
        python3 "$CLASSES" "$COMPARE" "$ERR"
        echo
        echo "== unresolved-symbol NAME SETS, both directions =="
        python3 - "$COMPARE" "$ERR" <<'PY'
import re, sys
pat = re.compile(r'`([^`]+)`')
def names(path):
    s = set()
    n = 0
    for line in open(path, encoding='utf-8', errors='replace'):
        n += 1
        if 'unresolved' in line or 'undefined' in line or 'dangling' in line:
            s.update(pat.findall(line))
    return s, n
b, bn = names(sys.argv[1]); a, an = names(sys.argv[2])
print("  population: %d line(s) before, %d after" % (bn, an))
print("  before-only (%d): %s" % (len(b - a), sorted(b - a)[:60]))
print("  after-only  (%d): %s" % (len(a - b), sorted(a - b)[:60]))
print("  in both     (%d)" % len(b & a))
PY
        sort -u "$COMPARE" > "$OUT/.cmp-before.txt"
        sort -u "$ERR" > "$OUT/.cmp-after.txt"
        echo
        echo "  lines only in the NEW run:  $(comm -13 "$OUT/.cmp-before.txt" "$OUT/.cmp-after.txt" | wc -l)"
        comm -13 "$OUT/.cmp-before.txt" "$OUT/.cmp-after.txt" | head -20 | sed 's/^/    /'
        echo "  lines only in the OLD run:  $(comm -23 "$OUT/.cmp-before.txt" "$OUT/.cmp-after.txt" | wc -l)"
        comm -23 "$OUT/.cmp-before.txt" "$OUT/.cmp-after.txt" | head -20 | sed 's/^/    /'
    fi
fi

echo
echo "=============================== CORPUS BASELINE ==============================="
echo "  label            $LABEL"
echo "  sigil md5        $(md5sum "$SIGIL" | cut -d' ' -f1)"
echo "  corpus rev       $(cd "$CORPUS" && git rev-parse --short HEAD 2>/dev/null)  dirty $(cd "$CORPUS" && git status --porcelain 2>/dev/null | wc -l)"
echo "  entry            $ENTRY"
echo "  generated set    $READY_STATE  ($((NWANT - NMISS))/$NWANT named path(s) present, $NGENDIAG run complaint(s))"
echo "  sigil exit       $RC"
echo "  DIAGNOSTICS      $NERR   over a stream of $NERR line(s)"
if [ "$READY_STATE" = "READY" ]; then
    echo "  RESULT           BASELINE"
else
    echo "  RESULT           NOT A BASELINE ($READY_STATE)"
fi
echo "==============================================================================="
echo "CORPUS-BASELINE-END rc=0 classes_rc=$CRC"
