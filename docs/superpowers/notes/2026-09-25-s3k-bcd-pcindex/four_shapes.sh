#!/usr/bin/env bash
# four_shapes.sh <tag> <sigil> <emit_sound_blob>: build all four aeon shapes in this
# parcel's AEON_DIR with the given tools; copy each ROM out; CRC32 (zlib) + size.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex
A=/home/volence/sonic_hacks/.aeon-bcd-pcindex
TAG="$1"; SIG="$2"; EMIT="$3"
OUT=$S/aeon/$TAG; rm -rf "$OUT"; mkdir -p "$OUT"
LOG=$S/logs/four-$TAG.log
{
  echo "aeon HEAD: $(git -C "$A" rev-parse HEAD)  status lines: $(git -C "$A" status --porcelain | wc -l)"
  echo "sigil md5: $(md5sum "$SIG" | cut -d' ' -f1)  emit md5: $(md5sum "$EMIT" | cut -d' ' -f1)"
  "$SIG" --version | sed -n 1,3p
  for shape in "sonic4:" "sonic4-debug:DEBUG=1" "demo:" "demo-debug:DEBUG=1"; do
    name=${shape%%:*}; envv=${shape#*:}
    arg=""; case $name in demo*) arg=demo;; esac
    rm -f "$A"/s4.bin "$A"/s4.debug.bin "$A"/demo.bin "$A"/demo.debug.bin
    ( cd "$A" && env $envv SIGIL_BUILD="$SIG" SIGIL_EMIT="$EMIT" NO_LINT=1 ./build.sh $arg ) > "$OUT/$name.build.log" 2>&1
    echo "$name build rc=$?"
    for f in s4.bin s4.debug.bin demo.bin demo.debug.bin; do
      [ -f "$A/$f" ] && cp "$A/$f" "$OUT/$name.$f" && echo "  produced $f"
    done
  done
  cd "$OUT" && for f in *.bin; do python3 -c 'import zlib,sys;d=open(sys.argv[1],"rb").read();print("%-28s crc32 %08x size %d"%(sys.argv[1],zlib.crc32(d)&0xffffffff,len(d)))' "$f"; done
} > "$LOG" 2>&1
echo FOUR_END_$TAG >> "$LOG"
