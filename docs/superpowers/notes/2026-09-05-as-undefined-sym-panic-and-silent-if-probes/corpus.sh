#!/bin/bash
# Corpus decomposition for the AS-UNDEFINED-SYM-PANIC-AND-SILENT-IF parcel.
#
# Runs the plain `sigil <root.asm>` path -- the exact CLI seam both faults live
# on -- over s2disasm's root, with the pre-change and post-change assemblers,
# and decomposes the diagnostics by class. A falling count is evidence noise was
# removed, never that correctness improved; a class that ROSE or APPEARED is the
# thing to read.
set -u
HERE=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a159ca0b948688d11
BEFORE=$HERE/.runlogs/sigil-before
AFTER=$HERE/.target-land/release/sigil
CORPUS=/home/volence/sonic_hacks/.parcel-undefsym-corpus-a159ca0b
OUT=$HERE/.runlogs/corpus
mkdir -p "$OUT/before" "$OUT/after"

echo "before md5: $(md5sum $BEFORE)"
echo "after  md5: $(md5sum $AFTER)"
echo "corpus rev: $(git -C $CORPUS rev-parse HEAD)"
echo "corpus dirty paths: $(git -C $CORPUS status --porcelain | wc -l)"

echo
echo "== freshness witness: each binary answers the two parcel probes =="
P=/tmp/claude-1000/-home-volence-sonic-hacks-sigil/1a93ba92-b503-43b3-8939-b5973f7954ac/scratchpad/p
# The exit status is captured from the BINARY, not from a pipeline: piping to
# `head` makes `$?` head's status, which reported exit=0 for a run that exits 1.
for w in BEFORE AFTER; do
  BIN=$BEFORE; [ "$w" = AFTER ] && BIN=$AFTER
  o=$("$BIN" "$P/if_undef.asm" --hex 2>&1); rc=$?
  echo "  $w if_undef : exit=$rc : $(echo "$o" | head -1)"
  o=$("$BIN" "$P/jsr_undef.asm" --hex 2>&1); rc=$?
  echo "  $w jsr_undef: exit=$rc : $(echo "$o" | head -2 | tr '\n' ' ')"
done

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
python3 "$HERE/docs/superpowers/notes/2026-09-05-as-undefined-sym-panic-and-silent-if-probes/classes.py" \
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
print("  in both     (%d)" % len(b & a))
PY

echo
echo "== every AFTER diagnostic line absent from BEFORE (new text) =="
sort -u "$OUT/before/s2.err" > "$OUT/before/uniq.txt"
sort -u "$OUT/after/s2.err"  > "$OUT/after/uniq.txt"
comm -13 "$OUT/before/uniq.txt" "$OUT/after/uniq.txt" | head -40
echo "  (count: $(comm -13 $OUT/before/uniq.txt $OUT/after/uniq.txt | wc -l))"
echo
echo "== every BEFORE diagnostic line absent from AFTER (lost text) =="
comm -23 "$OUT/before/uniq.txt" "$OUT/after/uniq.txt" | head -40
echo "  (count: $(comm -23 $OUT/before/uniq.txt $OUT/after/uniq.txt | wc -l))"
echo CORPUS-END-MARKER
