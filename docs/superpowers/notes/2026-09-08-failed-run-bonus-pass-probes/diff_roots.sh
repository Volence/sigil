#!/usr/bin/env bash
# Both-direction diagnostic SET comparison, plus per-class decomposition.
set -u
S=/home/volence/sonic_hacks/.scratch/bonus-pass
A="$S/logs/base"; B="$S/logs/cut"
D="$S/logs/diff"; mkdir -p "$D"

classify() {
  # message CLASS: drop the file(line) prefix, blank out backtick-quoted
  # identifiers and bare numbers so `unresolved symbol `Foo`` and
  # `unresolved symbol `Bar`` land in one class.
  sed -E 's/^[^:]*\([0-9]+\): //' \
  | sed -E 's/`[^`]*`/`X`/g' \
  | sed -E 's/\b(0x)?[0-9]+\b/N/g'
}

for name in s1 s2 s2mompass sk s3 s4legacy sce batman mdos; do
  ea=$(cat "$A/$name.exit"); eb=$(cat "$B/$name.exit")
  # exact-line multiset compare, both directions
  sort "$A/$name.err" > "$D/$name.a.sorted"
  sort "$B/$name.err" > "$D/$name.b.sorted"
  onlya=$(comm -23 "$D/$name.a.sorted" "$D/$name.b.sorted" | wc -l)
  onlyb=$(comm -13 "$D/$name.a.sorted" "$D/$name.b.sorted" | wc -l)
  comm -23 "$D/$name.a.sorted" "$D/$name.b.sorted" > "$D/$name.only-base.txt"
  comm -13 "$D/$name.a.sorted" "$D/$name.b.sorted" > "$D/$name.only-cut.txt"
  # stdout compare
  if cmp -s "$A/$name.out" "$B/$name.out"; then so=same; else so=DIFFER; fi
  # class sets
  classify < "$A/$name.err" | sort | uniq -c | sort -rn > "$D/$name.a.classes"
  classify < "$B/$name.err" | sort | uniq -c | sort -rn > "$D/$name.b.classes"
  ca=$(awk '{$1="";print}' "$D/$name.a.classes" | sort -u)
  cb=$(awk '{$1="";print}' "$D/$name.b.classes" | sort -u)
  newclass=$(comm -13 <(echo "$ca") <(echo "$cb") | sed '/^$/d')
  goneclass=$(comm -23 <(echo "$ca") <(echo "$cb") | sed '/^$/d')
  echo "== $name  exit base=$ea cut=$eb  stdout=$so  lines base=$(wc -l < "$A/$name.err") cut=$(wc -l < "$B/$name.err")"
  echo "   only-in-base(vanished): $onlya   only-in-cut(appeared): $onlyb"
  if [ -n "$newclass" ]; then echo "   NEW CLASSES:"; echo "$newclass" | sed 's/^/     + /'; fi
  if [ -n "$goneclass" ]; then echo "   VANISHED CLASSES:"; echo "$goneclass" | sed 's/^/     - /'; fi
done
