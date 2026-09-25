#!/usr/bin/env bash
# mk_trees.sh: pristine skdisasm (git archive at the census revision), a luaref copy
# where the stock buildSK.lua runs, and a gen tree = pristine + the generated PCM/DAC
# inputs + the census's wrapper root.
set -eu
S=/home/volence/sonic_hacks/.scratch/s3k-codepage
rm -rf "$S/trees/sk-pristine" "$S/trees/sk-luaref" "$S/trees/sk-gen"
mkdir -p "$S/trees/sk-pristine"
tar -x -C "$S/trees/sk-pristine" -f "$S/sk.tar"
cp -a "$S/trees/sk-pristine" "$S/trees/sk-luaref"
( cd "$S/trees/sk-luaref" && lua buildSK.lua ) > "$S/logs/luaref.log" 2>&1; echo "luaref rc=$?"
( cd "$S/trees/sk-luaref" && md5sum skbuilt.bin && python3 -c 'import zlib;d=open("skbuilt.bin","rb").read();print("crc32=%08x size=%d"%(zlib.crc32(d)&0xffffffff,len(d)))' )
cp -a "$S/trees/sk-pristine" "$S/trees/sk-gen"
for d in Sound/DAC/generated Sound/PCM/generated; do
  mkdir -p "$S/trees/sk-gen/$d"
  cp -a "$S/trees/sk-luaref/$d/." "$S/trees/sk-gen/$d/"
done
printf 'Sonic3_Complete = 0\n\tinclude "sonic3k.asm"\n' > "$S/trees/sk-gen/wrapper.asm"
echo "sk-gen: $(find "$S/trees/sk-gen" -type f | wc -l) files"
diff -rq "$S/trees/sk-pristine" "$S/trees/sk-gen" | sed 's|/[^/]*$||' | sort | uniq -c
echo MKTREES_END
