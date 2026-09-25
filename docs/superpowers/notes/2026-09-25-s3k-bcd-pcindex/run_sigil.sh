#!/usr/bin/env bash
# run_sigil.sh <tag> <sigil binary>: assemble the sk-gen wrapper root with the given
# sigil, the census's argument list; record provenance, exit, streams, sorted rows.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex
TAG="$1"
SIGIL="$2"
OUT=$S/runs/$TAG
rm -rf "$OUT"; mkdir -p "$OUT"
{
  echo "sigil: $SIGIL"
  echo "sigil md5: $(md5sum "$SIGIL" | cut -d' ' -f1)"
  echo "sigil rev: $("$SIGIL" --version | sed -n 2p)"
  echo "tree: $S/trees/sk-gen root: wrapper.asm"
} > "$OUT/provenance"
( cd "$S/trees/sk-gen" && "$SIGIL" wrapper.asm -o "$OUT/image.bin" -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before ) > "$OUT/stdout" 2> "$OUT/stderr"
echo "SIGIL_EXIT=$?" > "$OUT/exit"
echo "stderr lines: $(wc -l < "$OUT/stderr")" >> "$OUT/exit"
grep -E '(error|warning):' "$OUT/stderr" | sort > "$OUT/rows"
echo "rows: $(wc -l < "$OUT/rows")" >> "$OUT/exit"
cat "$OUT/provenance" "$OUT/exit"
echo RUN_END_$TAG
