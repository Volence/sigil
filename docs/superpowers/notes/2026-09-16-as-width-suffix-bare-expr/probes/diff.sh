#!/usr/bin/env bash
# Differential: asl vs sigil on one probe file. Compares BYTES, not listings.
. /home/volence/sonic_hacks/.wt-width-suffix/docs/superpowers/notes/asl-reference/asl_ref.sh || exit $?
SIGIL=/home/volence/sonic_hacks/.scratch/width-suffix/target/release/sigil
src="$1"; base="${src%.asm}"
rm -f "$base.p" "$base.asl.bin" "$base.sigil.bin"
asl_run -xx -n -q -A -L -U -i . -o "$base.p" "$src" || { echo "ASL FAILED on $src"; exit 1; }
"$ASLDIR/p2bin" "$base.p" "$base.asl.bin" >/dev/null || exit 1
"$SIGIL" "$src" -o "$base.sigil.bin" || { echo "SIGIL FAILED on $src"; exit 1; }
if cmp -s "$base.asl.bin" "$base.sigil.bin"; then
  echo "BYTE-IDENTICAL  $src  ($(stat -c%s "$base.asl.bin") bytes, crc32 $(python3 -c "import zlib,sys;print('%08x'%(zlib.crc32(open(sys.argv[1],'rb').read())&0xffffffff))" "$base.asl.bin"))"
else
  echo "DIVERGENT  $src"
  echo "  asl:   $(stat -c%s "$base.asl.bin") bytes"
  echo "  sigil: $(stat -c%s "$base.sigil.bin") bytes"
  cmp -l "$base.asl.bin" "$base.sigil.bin" | head -20
  exit 1
fi
