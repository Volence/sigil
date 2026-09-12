#!/usr/bin/env bash
# usage: shape.sh <tag> <game> <debug0|1> [fast0|1]   (env SIGIL_BIN_DIR selects the binaries, TREE the aeon tree)
set -u
S=/home/volence/sonic_hacks/.scratch/link-zero-byte-move
TREE=${TREE:-/home/volence/sonic_hacks/.sigil-zbm-aeon-cae58661}
tag=$1; game=$2; dbg=$3; fast=${4:-1}
BIN=${SIGIL_BIN_DIR:-$S/target/release}
if [[ $game == sonic4 ]]; then rom=s4; else rom=$game; fi
[[ $dbg == 1 ]] && rom=$rom.debug
log=$S/runs/$tag.$game.d$dbg.f$fast.log
cd "$TREE" || exit 9
rm -f "$rom.bin"
start=$(date +%T)
if [[ $dbg == 1 ]]; then
  FAST=$fast DEBUG=1 SIGIL_BUILD=$BIN/sigil SIGIL_EMIT=$BIN/emit_sound_blob ./build.sh "$game" > "$log" 2>&1
else
  FAST=$fast SIGIL_BUILD=$BIN/sigil SIGIL_EMIT=$BIN/emit_sound_blob ./build.sh "$game" > "$log" 2>&1
fi
rc=$?
end=$(date +%T)
if [[ -f $rom.bin ]]; then
  cp "$rom.bin" "$S/roms/$tag.$rom.f$fast.bin"
  [[ -f $rom.lst ]] && cp "$rom.lst" "$S/roms/$tag.$rom.f$fast.lst"
  crc=$(python3 -c "import zlib,sys;d=open(sys.argv[1],'rb').read();print('%08X %d'%(zlib.crc32(d),len(d)))" "$rom.bin")
else crc="NO-ROM"; fi
echo "$tag $game debug=$dbg fast=$fast rc=$rc rom=$rom.bin $crc [$start..$end] log=$log"
