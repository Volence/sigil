#!/bin/zsh
# Corpus decomposition for the AS-REGISTER-DIAGNOSTIC-RESIDUE parcel.
set -u
BEFORE=/home/volence/sonic_hacks/.reg-diag-before/.target-land/release/sigil
AFTER=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-aaae9c7d6e2503586/.target-land/release/sigil
CORPUS=/home/volence/sonic_hacks/.s2-reg-diag
OUT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-aaae9c7d6e2503586/.runlogs/corpus
HERE=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-aaae9c7d6e2503586
mkdir -p $OUT/before $OUT/after

echo "before md5: $(md5sum $BEFORE)"
echo "after  md5: $(md5sum $AFTER)"
echo "corpus rev: $(git -C $CORPUS rev-parse HEAD)"
echo "corpus dirty paths: $(git -C $CORPUS status --porcelain | wc -l)"

echo
echo "== freshness witness for the BEFORE binary: the four probes from the brief =="
P=/tmp/claude-1000/-home-volence-sonic-hacks-sigil/1a93ba92-b503-43b3-8939-b5973f7954ac/scratchpad/probes
cd $P
printf '\tcpu 68000\nig function p,$100\n\tdc.l ig(a0)\n' > w1.asm
printf '\tcpu 68000\nus function p,p+1\n\tdc.l us(a0)\n' > w2.asm
printf '\tcpu 68000\n\tdc.l a0+1\n' > w3.asm
printf '\tcpu 68000\n\tdc.l a0\n' > w4.asm
for f in w1 w2 w3 w4; do
  echo "  BEFORE $f: $($BEFORE $f.asm --hex 2>&1 | head -1)"
done

echo
echo "== corpus run =="
cd $CORPUS
$BEFORE s2.asm > $OUT/before/s2.out 2> $OUT/before/s2.err
echo "before exit=$?  stderr lines=$(wc -l < $OUT/before/s2.err)"
$AFTER s2.asm > $OUT/after/s2.out 2> $OUT/after/s2.err
echo "after  exit=$?  stderr lines=$(wc -l < $OUT/after/s2.err)"

echo
echo "== located vs locationless split, both runs =="
for w in before after; do
  tot=$(wc -l < $OUT/$w/s2.err)
  loc=$(grep -cE '^.*\([0-9]+\): (error|warning): ' $OUT/$w/s2.err)
  echo "  $w: total=$tot  with file(line)=$loc  without=$((tot-loc))"
done
echo "  lines WITHOUT a source location, before:"
grep -vE '^.*\([0-9]+\): (error|warning): ' $OUT/before/s2.err | sort | uniq -c | sort -rn | head -20
echo "  lines WITHOUT a source location, after:"
grep -vE '^.*\([0-9]+\): (error|warning): ' $OUT/after/s2.err | sort | uniq -c | sort -rn | head -20

echo
echo "== join and class table =="
JOIN=$HERE/docs/superpowers/notes/2026-09-05-s2-top-blocks-decompose-probes/join_source.py
python3 $JOIN $OUT/before/s2.err $CORPUS > $OUT/before/joined.tsv 2> $OUT/before/join.err
python3 $JOIN $OUT/after/s2.err  $CORPUS > $OUT/after/joined.tsv 2> $OUT/after/join.err
echo "  before unparsed: $(tail -1 $OUT/before/join.err)"
echo "  after  unparsed: $(tail -1 $OUT/after/join.err)"
python3 $HERE/docs/superpowers/notes/2026-09-05-as-jmptos-518-block-probes/classtable.py $OUT/before/joined.tsv $OUT/after/joined.tsv

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
sort -u $OUT/before/s2.err > $OUT/before/uniq.txt
sort -u $OUT/after/s2.err  > $OUT/after/uniq.txt
comm -13 $OUT/before/uniq.txt $OUT/after/uniq.txt | head -40
echo "  (count: $(comm -13 $OUT/before/uniq.txt $OUT/after/uniq.txt | wc -l))"

echo
echo "== register message rows in the AFTER run =="
grep -c "is a register, not a value" $OUT/after/s2.err
echo "CORPUS-END-MARKER"
