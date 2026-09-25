#!/usr/bin/env bash
# run_sigil.sh <tag> : assemble the sk-gen wrapper root with this parcel's sigil,
# the census's argument list, and record provenance, exit, streams.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-codepage
SIGIL=$S/target/release/sigil
TAG="$1"
OUT=$S/runs/$TAG
rm -rf "$OUT"; mkdir -p "$OUT"
{
  echo "sigil md5: $(md5sum "$SIGIL" | cut -d' ' -f1)"
  echo "sigil rev: $("$SIGIL" --version | sed -n 2p)"
  echo "tree: $S/trees/sk-gen root: wrapper.asm"
  echo "uptime: $(uptime)"
} > "$OUT/provenance"
( cd "$S/trees/sk-gen" && "$SIGIL" wrapper.asm -o "$OUT/image.bin" -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before ) > "$OUT/stdout" 2> "$OUT/stderr"
echo "SIGIL_EXIT=$?" > "$OUT/exit"
echo "stderr lines: $(wc -l < "$OUT/stderr")" >> "$OUT/exit"
grep -E '(error|warning):' "$OUT/stderr" | sort > "$OUT/rows"
echo "rows: $(wc -l < "$OUT/rows")" >> "$OUT/exit"
cat "$OUT/provenance" "$OUT/exit" "$OUT/stdout"
echo RUN_END_$TAG
