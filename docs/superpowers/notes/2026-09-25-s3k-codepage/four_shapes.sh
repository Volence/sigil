#!/usr/bin/env bash
# four_shapes.sh <tag> <target dir>: build all four aeon shapes in this parcel's
# AEON_DIR with <target>/release/sigil and emit_sound_blob; copy each ROM out.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-codepage
A=/home/volence/sonic_hacks/.aeon-s3k-codepage
TAG="$1"; T="$2"
OUT=$S/aeon/$TAG; rm -rf "$OUT"; mkdir -p "$OUT"
LOG=$S/logs/four-$TAG.log
{
  echo "aeon HEAD: $(git -C "$A" rev-parse HEAD)"
  echo "sigil md5: $(md5sum "$T/release/sigil" | cut -d' ' -f1)"
  "$T/release/sigil" --version | sed -n 1,3p
  for shape in "sonic4:" "sonic4-debug:DEBUG=1" "demo:" "demo-debug:DEBUG=1"; do
    name=${shape%%:*}; envv=${shape#*:}
    arg=""; case $name in demo*) arg=demo;; esac
    rm -f "$A"/s4.bin "$A"/s4.debug.bin "$A"/demo.bin "$A"/demo.debug.bin
    ( cd "$A" && env $envv SIGIL_BUILD="$T/release/sigil" SIGIL_EMIT="$T/release/emit_sound_blob" NO_LINT=1 ./build.sh $arg ) > "$OUT/$name.build.log" 2>&1
    echo "$name build rc=$?"
    for f in s4.bin s4.debug.bin demo.bin demo.debug.bin; do
      [ -f "$A/$f" ] && cp "$A/$f" "$OUT/$name.$f" && echo "  produced $f"
    done
  done
  cd "$OUT" && for f in *.bin; do python3 -c 'import zlib,sys;d=open(sys.argv[1],"rb").read();print("%-28s crc32 %08x size %d"%(sys.argv[1],zlib.crc32(d)&0xffffffff,len(d)))' "$f"; done
} > "$LOG" 2>&1
echo FOUR_END_$TAG >> "$LOG"
cat "$LOG"
