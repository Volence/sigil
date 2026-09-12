#!/usr/bin/env bash
# emp_compare.sh BASE NEW : the .emp tier's output, before and after, byte for byte.
# Every committed .emp file, and a copy of each with a garbage line appended (so
# every input yields at least one located diagnostic), through `sigil parse` and
# `sigil emp`, on both binaries. Compares exit status, stdout and stderr.
BASE="$1"; NEW="$2"
S=/home/volence/sonic_hacks/.scratch/as-macro-diag
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a46ef34b369e42312
OUT="$S/emp"
rm -rf "$OUT"; mkdir -p "$OUT/in" "$OUT/base" "$OUT/new"
echo "base md5: $(md5sum "$BASE" | cut -d' ' -f1)   new md5: $(md5sum "$NEW" | cut -d' ' -f1)"
cd "$W" || exit 1
n=0; same=0; diff=0; located=0
while IFS= read -r f; do
  key=$(echo "$f" | tr '/' '_')
  cp "$f" "$OUT/in/$key"
  { cat "$f"; printf '\n@@@ this line is not emp @@@\n'; } > "$OUT/in/bad_$key"
done < <(git ls-files '*.emp')
cd "$OUT/in" || exit 1
for inp in *.emp; do
  for cmd in parse emp; do
    for side in base new; do
      bin=$BASE; [ "$side" = new ] && bin=$NEW
      if [ "$cmd" = emp ]; then
        # One output path for both sides, because `sigil emp` echoes it on stdout.
        rm -f "$OUT/cur.bin"
        "$bin" emp "$inp" -o "$OUT/cur.bin" > "$OUT/$side/$inp.$cmd.out" 2> "$OUT/$side/$inp.$cmd.err"
        rc=$?
        [ -f "$OUT/cur.bin" ] && mv "$OUT/cur.bin" "$OUT/$side/$inp.bin"
        (exit $rc)
      else
        "$bin" parse "$inp" > "$OUT/$side/$inp.$cmd.out" 2> "$OUT/$side/$inp.$cmd.err"
      fi
      echo "exit=$?" >> "$OUT/$side/$inp.$cmd.out"
    done
    n=$((n+1))
    if cmp -s "$OUT/base/$inp.$cmd.out" "$OUT/new/$inp.$cmd.out" && cmp -s "$OUT/base/$inp.$cmd.err" "$OUT/new/$inp.$cmd.err"; then
      same=$((same+1))
    else
      diff=$((diff+1)); echo "DIFF $cmd $inp"
    fi
    if grep -qE "$inp:[0-9]+:[0-9]+" "$OUT/new/$inp.$cmd.out" "$OUT/new/$inp.$cmd.err"; then located=$((located+1)); fi
  done
done
for b in "$OUT"/base/*.bin; do
  nb="$OUT/new/$(basename "$b")"
  if [ -f "$nb" ] && ! cmp -s "$b" "$nb"; then echo "IMAGE DIFF $(basename "$b")"; diff=$((diff+1)); fi
done
echo "runs compared: $n   identical: $same   different: $diff   runs whose output carries a path:line:col location: $located"
echo "images written: base $(ls "$OUT"/base/*.bin 2>/dev/null | wc -l)  new $(ls "$OUT"/new/*.bin 2>/dev/null | wc -l)"
