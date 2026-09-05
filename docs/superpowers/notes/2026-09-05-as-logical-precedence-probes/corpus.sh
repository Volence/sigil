#!/bin/bash
# Corpus and aeon gate for the AS-ANDAND-PRECEDENCE parcel.
#
# Plain `sigil <root.asm>` over s2disasm at e45ebf33 and over aeon's three
# AS-routed roots, before and after, compared on BYTES and on diagnostics.
#
# A precedence change can alter any expression in the tree, so a moved
# diagnostic count is not automatically an improvement and byte identity is the
# gate, not the count. The freshness witness below is load-bearing: it proves
# the two binaries are actually different programs on this parcel's own subject,
# so a byte-identical corpus reads as INERTNESS rather than as a build that did
# not happen.
set -u
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a3c8b0be4dfd2ab68
BEFORE=$W/.runlogs/sigil-before
AFTER=$W/.runlogs/sigil-after
CORPUS=/home/volence/sonic_hacks/s2disasm-andand-a3c8
OUT=$W/.runlogs/corpus
mkdir -p "$OUT/before" "$OUT/after"

echo "before md5: $(md5sum $BEFORE)"
echo "after  md5: $(md5sum $AFTER)"
echo "corpus rev: $(cd $CORPUS && git rev-parse HEAD)"
echo "corpus dirty paths: $(cd $CORPUS && git status --porcelain | wc -l)"

echo
echo "== freshness witness: each binary answers this parcel's own probe =="
# Exit status comes from the BINARY, not from a pipeline.
P=$W/.runlogs/witness/w.asm
o=$("$BEFORE" "$P" --hex 2>&1); rc=$?
echo "  BEFORE w.asm: exit=$rc : $(printf '%s' "$o" | head -1)"
o=$("$AFTER" "$P" --hex 2>&1); rc=$?
echo "  AFTER  w.asm: exit=$rc : $(printf '%s' "$o" | head -1)"
echo "  asl for the same three lines: 00 09 00"

echo
echo "== corpus run =="
cd "$CORPUS" || exit 9
"$BEFORE" s2.asm -o "$OUT/before/s2.bin" > "$OUT/before/s2.out" 2> "$OUT/before/s2.err"
echo "before exit=$?  stdout lines=$(wc -l < $OUT/before/s2.out)  stderr lines=$(wc -l < $OUT/before/s2.err)"
"$AFTER" s2.asm -o "$OUT/after/s2.bin" > "$OUT/after/s2.out" 2> "$OUT/after/s2.err"
echo "after  exit=$?  stdout lines=$(wc -l < $OUT/after/s2.out)  stderr lines=$(wc -l < $OUT/after/s2.err)"

echo
echo "== corpus BYTE identity =="
for w in before after; do
  if [ -f "$OUT/$w/s2.bin" ]; then
    echo "  $w: $(cksum -a crc32 -a md5 "$OUT/$w/s2.bin" 2>/dev/null || md5sum "$OUT/$w/s2.bin")  size=$(stat -c%s "$OUT/$w/s2.bin")"
  else
    echo "  $w: NO OUTPUT FILE (the corpus does not assemble to completion; diagnostics are the gate)"
  fi
done
if [ -f "$OUT/before/s2.bin" ] && [ -f "$OUT/after/s2.bin" ]; then
  cmp -s "$OUT/before/s2.bin" "$OUT/after/s2.bin" && echo "  BYTE-IDENTICAL" || echo "  BYTES DIFFER"
fi

echo
echo "== stdout identity =="
cmp -s "$OUT/before/s2.out" "$OUT/after/s2.out" && echo "  stdout IDENTICAL" || { echo "  stdout DIFFERS"; diff "$OUT/before/s2.out" "$OUT/after/s2.out" | head -20; }

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
print("  in both     (%d): %s" % (len(b & a), sorted(b & a)[:8]))
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

echo
echo "== aeon: the three AS-routed roots, asserted not assumed =="
for r in /home/volence/sonic_hacks/aeon/games/sonic4/game_root.asm \
         /home/volence/sonic_hacks/aeon/games/demo/game_root.asm \
         /home/volence/sonic_hacks/aeon/engine/debug/debugger.asm; do
  b=$(basename "$(dirname "$r")")-$(basename "$r" .asm)
  cd "$(dirname "$r")" || exit 9
  for w in before after; do
    bin=$W/.runlogs/corpus/$w
    if [ "$w" = before ]; then E=$BEFORE; else E=$AFTER; fi
    "$E" "$(basename "$r")" -o "$bin/$b.bin" > "$bin/$b.out" 2> "$bin/$b.err"
    echo "  $w $b: exit=$? stdout=$(wc -l < $bin/$b.out) stderr=$(wc -l < $bin/$b.err) bytes=$(stat -c%s $bin/$b.bin 2>/dev/null || echo none)"
  done
  # `cmp -s` exits 2 when a file is MISSING, which is not the same answer as
  # "the two differ" and must not be printed as one. A root that exits non-zero
  # writes no `.bin` at all, and reporting that as a byte divergence would be a
  # check that fires on correct code.
  for ext in bin out err; do
    if [ ! -e "$OUT/before/$b.$ext" ] && [ ! -e "$OUT/after/$b.$ext" ]; then
      echo "    $b.$ext ABSENT IN BOTH (this root emits no such file; nothing to compare)"
    elif [ ! -e "$OUT/before/$b.$ext" ] || [ ! -e "$OUT/after/$b.$ext" ]; then
      echo "    $b.$ext PRESENT IN ONLY ONE RUN <== read this"
    elif cmp -s "$OUT/before/$b.$ext" "$OUT/after/$b.$ext"; then
      echo "    $b.$ext IDENTICAL"
    else
      echo "    $b.$ext DIFFERS <== read this"
      diff "$OUT/before/$b.$ext" "$OUT/after/$b.$ext" 2>/dev/null | head -10
    fi
  done
done
echo CORPUS-END-MARKER
