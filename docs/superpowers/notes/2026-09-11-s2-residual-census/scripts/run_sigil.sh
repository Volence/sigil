#!/usr/bin/env bash
# run_sigil.sh <corpus-dir> <out-dir>
# Assemble <corpus-dir>/s2.asm with the census sigil binary; record exit, time,
# stdout, stderr (the diagnostic stream), image md5/size when one is written.
set -u
S=/home/volence/sonic_hacks/.scratch/s2-residual-census
SIGIL=$S/target/release/sigil
CORPUS="$1"; OUT="$2"
mkdir -p "$OUT"
echo "sigil md5: $(md5sum "$SIGIL" | cut -d' ' -f1)" > "$OUT/provenance"
echo "corpus: $CORPUS" >> "$OUT/provenance"
cd "$CORPUS" || exit 9
start=$(date +%s.%N)
"$SIGIL" s2.asm -o "$OUT/s2.sigil.bin" > "$OUT/stdout" 2> "$OUT/stderr"
rc=$?
end=$(date +%s.%N)
echo "SIGIL_EXIT=$rc" > "$OUT/exit"
echo "elapsed: $(echo "$end - $start" | bc)" >> "$OUT/exit"
echo "stderr lines: $(wc -l < "$OUT/stderr")" >> "$OUT/exit"
echo "stdout lines: $(wc -l < "$OUT/stdout")" >> "$OUT/exit"
if [ -f "$OUT/s2.sigil.bin" ]; then
  echo "image: $(md5sum "$OUT/s2.sigil.bin" | cut -d' ' -f1) size=$(stat -c%s "$OUT/s2.sigil.bin")" >> "$OUT/exit"
else
  echo "image: NONE" >> "$OUT/exit"
fi
cat "$OUT/exit"
echo RUN_END
