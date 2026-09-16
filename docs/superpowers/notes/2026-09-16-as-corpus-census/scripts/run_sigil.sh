#!/usr/bin/env bash
# run_sigil.sh <tree> <root.asm> <out-tag> [extra sigil args...]
# Assemble <tree>/<root.asm> with this parcel's sigil; record exit, wall time,
# stdout, stderr (the diagnostic stream), image md5/CRC32/size when one is written.
set -u
S=/home/volence/sonic_hacks/.scratch/as-corpus-census
SIGIL=$S/target/release/sigil
TREE="$1"; ROOT="$2"; TAG="$3"; shift 3
OUT=$S/runs/$TAG
rm -rf "$OUT"; mkdir -p "$OUT"
{
  echo "sigil md5: $(md5sum "$SIGIL" | cut -d' ' -f1)"
  echo "tree: $TREE"
  echo "root: $ROOT"
  echo "args: $*"
  echo "uptime: $(uptime)"
} > "$OUT/provenance"
start=$(date +%s.%N)
( cd "$TREE" && "$SIGIL" "$ROOT" -o "$OUT/image.bin" "$@" ) > "$OUT/stdout" 2> "$OUT/stderr"
rc=$?
end=$(date +%s.%N)
{
  echo "SIGIL_EXIT=$rc"
  echo "elapsed_s: $(awk -v a=$start -v b=$end 'BEGIN{printf "%.2f", b-a}')"
  echo "stderr lines: $(wc -l < "$OUT/stderr")"
  echo "stdout lines: $(wc -l < "$OUT/stdout")"
  if [ -f "$OUT/image.bin" ]; then
    echo "image md5: $(md5sum "$OUT/image.bin" | cut -d' ' -f1)"
    echo "image crc32: $(python3 -c 'import zlib,sys;print("%08x"%(zlib.crc32(open(sys.argv[1],"rb").read())&0xffffffff))' "$OUT/image.bin")"
    echo "image size: $(stat -c%s "$OUT/image.bin")"
  else
    echo "image: NONE"
  fi
} > "$OUT/exit"
cat "$OUT/exit"
echo RUN_END_$TAG
