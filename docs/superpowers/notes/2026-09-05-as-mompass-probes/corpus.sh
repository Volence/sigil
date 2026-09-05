#!/bin/bash
# Corpus decomposition for the AS-MOMPASS parcel.
#
# Plain `sigil <root.asm>` over s2disasm's root at e45ebf33, before and after,
# decomposed by diagnostic class. A FALLING count is evidence noise was removed,
# never that correctness improved; a class that ROSE or APPEARED is the thing to
# read. The after-count is measured, never predicted by subtraction: closing
# MOMPASS lets the assembler reach code it previously abandoned, which can ADD
# rows as well as remove them.
set -u
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f
BEFORE=$W/.runlogs/sigil-before
AFTER=$W/.target-land/release/sigil
CORPUS=/home/volence/sonic_hacks/s2disasm-mompass-clean
OUT=$W/.runlogs/corpus
mkdir -p "$OUT/before" "$OUT/after"

echo "before md5: $(md5sum $BEFORE)"
echo "after  md5: $(md5sum $AFTER)"
echo "corpus rev: $(cd $CORPUS && git rev-parse HEAD)"
echo "corpus dirty paths: $(cd $CORPUS && git status --porcelain | wc -l)"

echo
echo "== freshness witness: each binary answers the parcel probe =="
# The exit status comes from the BINARY, not a pipeline: piping to head makes $?
# head's status.
P=$W/.mompassprobe/m_eq1.asm
o=$("$BEFORE" "$P" --hex 2>&1); rc=$?
echo "  BEFORE m_eq1: exit=$rc : $(printf '%s' "$o" | head -1)"
o=$("$AFTER" "$P" --hex 2>&1); rc=$?
echo "  AFTER  m_eq1: exit=$rc : $(printf '%s' "$o" | head -1)"

echo
echo "== corpus run =="
cd "$CORPUS" || exit 9
"$BEFORE" s2.asm > "$OUT/before/s2.out" 2> "$OUT/before/s2.err"
echo "before exit=$?  stderr lines=$(wc -l < $OUT/before/s2.err)"
"$AFTER" s2.asm > "$OUT/after/s2.out" 2> "$OUT/after/s2.err"
echo "after  exit=$?  stderr lines=$(wc -l < $OUT/after/s2.err)"

echo
echo "== located vs locationless split, both runs =="
for w in before after; do
  tot=$(wc -l < "$OUT/$w/s2.err")
  loc=$(grep -cE '^.*\([0-9]+\): (error|warning): ' "$OUT/$w/s2.err")
  echo "  $w: total=$tot  with file(line)=$loc  without=$((tot-loc))"
done

echo
echo "== class table (message text, normalised) =="
python3 "$W/docs/superpowers/notes/2026-09-05-as-undefined-sym-panic-and-silent-if-probes/classes.py" \
  "$OUT/before/s2.err" "$OUT/after/s2.err"

echo
echo "== unresolved-symbol NAME SETS, both directions =="
python3 - "$OUT/before/s2.err" "$OUT/after/s2.err" <<'PY'
import re, sys
pat = re.compile(r'`([^`]+)`')
def names(path):
    s = set()
    for line in open(path, encoding='utf-8', errors='replace'):
        if 'unresolved' in line or 'undefined' in line or 'dangling' in line:
            s.update(pat.findall(line))
    return s
b = names(sys.argv[1]); a = names(sys.argv[2])
print("  before-only (%d): %s" % (len(b - a), sorted(b - a)[:60]))
print("  after-only  (%d): %s" % (len(a - b), sorted(a - b)[:60]))
print("  in both     (%d): %s" % (len(b & a), sorted(b & a)[:60]))
PY

echo
sort -u "$OUT/before/s2.err" > "$OUT/before/uniq.txt"
sort -u "$OUT/after/s2.err"  > "$OUT/after/uniq.txt"
echo "== every AFTER diagnostic line absent from BEFORE (new text) =="
comm -13 "$OUT/before/uniq.txt" "$OUT/after/uniq.txt" | head -40
echo "  (count: $(comm -13 $OUT/before/uniq.txt $OUT/after/uniq.txt | wc -l))"
echo
echo "== every BEFORE diagnostic line absent from AFTER (lost text) =="
comm -23 "$OUT/before/uniq.txt" "$OUT/after/uniq.txt" | head -40
echo "  (count: $(comm -23 $OUT/before/uniq.txt $OUT/after/uniq.txt | wc -l))"
echo CORPUS-END-MARKER
