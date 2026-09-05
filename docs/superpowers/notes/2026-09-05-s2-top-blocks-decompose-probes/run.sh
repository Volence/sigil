#!/usr/bin/env bash
# Re-run the AS-S2-TOP-BLOCKS-DECOMPOSE measurement end to end.
#
#   run.sh <sigil-binary> <corpus-root> <out-dir>
#
# Defaults match the 2026-09-05 run: the corpus is a detached `s2disasm`
# worktree at e45ebf33 and the binary is a release `sigil` built from this
# repo. It prints the binary's md5 and both revisions FIRST, because a class
# total is only a fact about the instrument that produced it.
set -u
SIGIL="${1:?usage: run.sh <sigil-binary> <corpus-root> <out-dir>}"
CORPUS="${2:?}"
OUT="${3:?}"
HERE="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$OUT"

echo "sigil binary : $SIGIL"
md5sum "$SIGIL"
echo "sigil rev    : $(git -C "$HERE" rev-parse HEAD)"
echo "corpus rev   : $(git -C "$CORPUS" rev-parse HEAD)"
echo "corpus dirty : $(git -C "$CORPUS" status --porcelain | wc -l) path(s)"
date

( cd "$CORPUS" && "$SIGIL" s2.asm ) > "$OUT/s2.out" 2> "$OUT/s2.err"
echo "sigil exit=$?  rows=$(wc -l < "$OUT/s2.err")"
date

python3 "$HERE/join_source.py" "$OUT/s2.err" "$CORPUS" > "$OUT/joined.tsv"
echo "joined rows: $(wc -l < "$OUT/joined.tsv")"
echo "rows whose (file,line) had no source text: $(grep -c '<<UNRESOLVED>>' "$OUT/joined.tsv" || true)"

echo
echo "== every message class: rows / distinct sites =="
python3 - "$OUT/joined.tsv" <<'PY'
import collections, re, sys
h = collections.defaultdict(lambda: [0, set()])
for line in open(sys.argv[1]):
    f, ln, msg, src = line.rstrip('\n').split('\t', 3)
    k = re.sub(r'`[^`]*`', '`X`', msg)
    k = re.sub(r'cannot include .*', 'cannot include <file>: no such file', k)
    k = re.sub(r'unsupported form: .*', 'unsupported form: <insn>', k)
    h[k][0] += 1
    h[k][1].add((f, ln))
tot = 0
print("%-6s %-6s %s" % ("rows", "sites", "message"))
for k, (n, s) in sorted(h.items(), key=lambda kv: -kv[1][0]):
    print("%-6d %-6d %s" % (n, len(s), k))
    tot += n
print("TOTAL rows %d   TOTAL distinct sites %d" % (
    tot, len({tuple(l.split('\t')[:2]) for l in open(sys.argv[1])})))
PY

echo
for cls in "bad operand expression" "expected mnemonic, directive, or label"; do
  python3 "$HERE/classify.py" "$OUT/joined.tsv" "$cls"
  echo
done

echo "== the four-shape probe: sigil refuses all four, reference asl accepts all four =="
ASL=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl
WANT=61e672562465725a8c102288a7da9098
GOT=$(md5sum "$ASL" | cut -d' ' -f1)
if [ "$GOT" != "$WANT" ]; then
  echo "REFUSED: asl md5 $GOT != $WANT - not the reference build, skipping the probe."
else
  ( cd "$HERE" && "$SIGIL" nameless_shapes.asm; echo "sigil exit=$?" )
  ( cd "$HERE" && AS_MSGPATH="$(dirname "$ASL")" "$ASL" -q -A -L -U nameless_shapes.asm; echo "asl exit=$?" )
fi
