#!/usr/bin/env bash
# run_diag.sh <tree> <root.asm> <label> <binary-name>... : assemble <root.asm>
# in <tree> with each named binary from bin/, keep each run's diagnostic stream
# (stderr) as the row multiset, and print the exact-line multiset diff of every
# binary against the first one named.
set -u
S=/home/volence/sonic_hacks/.scratch/s2-as-small-features
TREE=$1; ROOT=$2; LABEL=$3; shift 3
first=""
for b in "$@"; do
  out="$S/runs/diag-$LABEL-$b"
  mkdir -p "$out"
  rm -f "$out/image.bin"
  (cd "$TREE" && "$S/bin/sigil-$b" "$ROOT" -o "$out/image.bin" > "$out/stdout" 2> "$out/stderr")
  rc=$?
  sort "$out/stderr" > "$out/rows.sorted"
  img="NONE"; [ -f "$out/image.bin" ] && img="$(md5sum < "$out/image.bin" | cut -c1-32) size=$(stat -c%s "$out/image.bin")"
  echo "$LABEL $b: exit=$rc sigil=$(md5sum < "$S/bin/sigil-$b" | cut -c1-32) rows=$(wc -l < "$out/stderr") stdout=$(wc -l < "$out/stdout") image=$img"
  if [ -z "$first" ]; then
    first=$b
  else
    echo "--- $LABEL rows only in $first (<) / only in $b (>):"
    diff "$S/runs/diag-$LABEL-$first/rows.sorted" "$out/rows.sorted" | /usr/bin/grep -E '^[<>]' || echo "    (row multisets identical)"
    echo "--- $LABEL stdout diff $first vs $b:"
    diff "$S/runs/diag-$LABEL-$first/stdout" "$out/stdout" | /usr/bin/grep -E '^[<>]' | head -20 || echo "    (stdout identical)"
  fi
done
echo DIAG_END
