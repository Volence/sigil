#!/usr/bin/env bash
# Re-run the AS-JMPTOS-518-BLOCK evidence.
#
#   run.sh <sigil-before> <sigil-after> <s2disasm-corpus> <out-dir>
#
# `sigil-before` is a sigil built from the commit BEFORE this parcel, and
# `sigil-after` one built from its tip. Both are needed: a corpus class table is
# a fact about the instrument, so the two runs must differ in nothing else.
#
# Every asl invocation below checks TWO things and reports both: the binary's
# md5, and the run's EXIT STATUS. The digest alone is not enough. A run that
# carries an error still prints a full byte column for the lines that did
# assemble, and those bytes are not this build's answer for them, so a non-zero
# exit disqualifies the whole listing rather than just the failing line. That is
# not hypothetical here: the first capture of `insn_operands.asm` exited 2 on an
# extra `jmp val("Foo")` line and its listing looked complete.
set -u
BEFORE="${1:?usage: run.sh <sigil-before> <sigil-after> <corpus> <out-dir>}"
AFTER="${2:?}"
CORPUS="${3:?}"
OUT="${4:?}"
HERE="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$OUT/before" "$OUT/after"

echo "sigil before : $BEFORE"
md5sum "$BEFORE"
echo "sigil after  : $AFTER"
md5sum "$AFTER"
echo "corpus rev   : $(git -C "$CORPUS" rev-parse HEAD)"
echo "corpus dirty : $(git -C "$CORPUS" status --porcelain | wc -l) path(s)"
date

echo
echo "== 1. the reference assembler on every probe =="
ASL=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl
WANT=61e672562465725a8c102288a7da9098
GOT=$(md5sum "$ASL" | cut -d' ' -f1)
if [ "$GOT" != "$WANT" ]; then
  echo "REFUSED: asl md5 $GOT is not the reference build $WANT. No expected value below is measured."
else
  for f in nest_args nest_fn nest_plain insn_operands jmptos disp_head; do
    ( cd "$HERE" && AS_MSGPATH="$(dirname "$ASL")" "$ASL" -q -A -L -U "$f.asm" > /dev/null 2>&1 )
    rc=$?
    echo "asl $f.asm exit=$rc"
    if [ "$rc" -ne 0 ]; then
      echo "  ^ NON-ZERO: this listing is NOT a source of expected bytes for ANY of its lines."
    fi
  done
  rm -f "$HERE"/*.p
fi

echo
echo "== 2. the inertness claim, on the PRE-PARCEL binary =="
echo "Every builtin-head operand below must be REFUSED by the before binary."
echo "A line that assembles is a counterexample and the parcel's inertness"
echo "argument for the shipping build is wrong."
( cd "$HERE" && "$BEFORE" inertness.asm; echo "before exit=$? (expect 1, with lines 18..25 diagnosed and 26..27 silent)" )
( cd "$HERE" && "$AFTER" inertness.asm; echo "after  exit=$? (expect 0)" )

echo
echo "== 3. the displacement control assembles to the same bytes on both =="
b=$( cd "$HERE" && "$BEFORE" disp_head.asm --hex | tr -d ' \n' )
a=$( cd "$HERE" && "$AFTER"  disp_head.asm --hex | tr -d ' \n' )
if [ "$b" = "$a" ]; then echo "IDENTICAL"; else echo "DIFFERENT (before=...${b: -20} after=...${a: -20})"; fi

echo
echo "== 4. the corpus, before and after =="
( cd "$CORPUS" && "$BEFORE" s2.asm > "$OUT/before/s2.out" 2> "$OUT/before/s2.err" )
echo "before exit=$?  rows=$(wc -l < "$OUT/before/s2.err")  stdout bytes=$(wc -c < "$OUT/before/s2.out")"
( cd "$CORPUS" && "$AFTER"  s2.asm > "$OUT/after/s2.out"  2> "$OUT/after/s2.err" )
echo "after  exit=$?  rows=$(wc -l < "$OUT/after/s2.err")   stdout bytes=$(wc -c < "$OUT/after/s2.out")"

JOIN="$HERE/../2026-09-05-s2-top-blocks-decompose-probes/join_source.py"
python3 "$JOIN" "$OUT/before/s2.err" "$CORPUS" > "$OUT/before/joined.tsv"
python3 "$JOIN" "$OUT/after/s2.err"  "$CORPUS" > "$OUT/after/joined.tsv"

echo
echo "== 5. the class table, both runs, with any class that ROSE or APPEARED flagged =="
python3 "$HERE/classtable.py" "$OUT/before/joined.tsv" "$OUT/after/joined.tsv"

echo
echo "== 6. the unresolved-symbol name sets, compared in BOTH directions =="
for d in before after; do
  grep -o "unresolved symbol \`[^\`]*\`" "$OUT/$d/joined.tsv" \
    | sed 's/.*`\(.*\)`/\1/' | sort -u > "$OUT/$d/unres.txt"
done
echo "newly unresolved (in after, not before):"
comm -13 "$OUT/before/unres.txt" "$OUT/after/unres.txt" | sed 's/^/  /'
echo "newly resolved (in before, not after):"
comm -23 "$OUT/before/unres.txt" "$OUT/after/unres.txt" | sed 's/^/  /'

echo
echo "== 7. the whole diagnostic multiset, line by line =="
sort "$OUT/before/s2.err" > "$OUT/b.sorted"
sort "$OUT/after/s2.err"  > "$OUT/a.sorted"
echo "lines present AFTER but not before: $(comm -13 "$OUT/b.sorted" "$OUT/a.sorted" | wc -l)"
comm -13 "$OUT/b.sorted" "$OUT/a.sorted" | sed 's/^/  /' | head -20
echo "lines present BEFORE but not after: $(comm -23 "$OUT/b.sorted" "$OUT/a.sorted" | wc -l)"
comm -23 "$OUT/b.sorted" "$OUT/a.sorted" | sed 's/.*error: //' | sort | uniq -c | sed 's/^/  /'
