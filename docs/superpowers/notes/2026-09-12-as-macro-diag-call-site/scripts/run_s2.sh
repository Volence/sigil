#!/usr/bin/env bash
# run_s2.sh TAG BINARY : assemble the s2disasm archive copy with a sigil binary.
# Writes logs/s2-TAG.stderr, logs/s2-TAG.stdout, out/s2-TAG.bin, and an END marker.
TAG="$1"; BIN="$2"
S=/home/volence/sonic_hacks/.scratch/as-macro-diag
mkdir -p "$S/out" "$S/logs"
cd "$S/corpus/s2disasm" || exit 1
{
  echo "binary md5: $(md5sum "$BIN" | cut -d' ' -f1)  path: $BIN"
  echo "corpus: $(pwd)  s2disasm HEAD e45ebf332f39987424ca3102e50c717628f71269 (git archive)"
  echo "start: $(date -Is)"
} > "$S/logs/s2-$TAG.meta"
rm -f "$S/out/s2-$TAG.bin"
"$BIN" s2.asm -o "$S/out/s2-$TAG.bin" > "$S/logs/s2-$TAG.stdout" 2> "$S/logs/s2-$TAG.stderr"
rc=$?
echo "SIGIL_EXIT=$rc" >> "$S/logs/s2-$TAG.meta"
echo "end: $(date -Is)" >> "$S/logs/s2-$TAG.meta"
if [ -f "$S/out/s2-$TAG.bin" ]; then
  echo "image: $(stat -c %s "$S/out/s2-$TAG.bin") bytes crc32 $(python3 -c "import zlib,sys;print('%08x'%zlib.crc32(open(sys.argv[1],'rb').read()))" "$S/out/s2-$TAG.bin")" >> "$S/logs/s2-$TAG.meta"
else
  echo "image: none" >> "$S/logs/s2-$TAG.meta"
fi
echo "END_MARKER" >> "$S/logs/s2-$TAG.meta"
