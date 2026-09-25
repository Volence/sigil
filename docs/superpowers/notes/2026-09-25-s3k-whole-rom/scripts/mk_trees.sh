#!/usr/bin/env bash
# mk_trees.sh: pristine skdisasm (git archive 2fcd861c), a luaref copy where the stock
# buildSK.lua runs three times (the reference ROM), and a gen tree = pristine + the
# generated PCM/DAC inputs + the census wrapper root.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
ID="python3 $S/scripts/ident.py"
rm -rf "$S/trees/sk-pristine" "$S/trees/sk-luaref" "$S/trees/sk-gen"
mkdir -p "$S/trees/sk-pristine"
tar -x -C "$S/trees/sk-pristine" -f "$S/sk.tar"
cp -a "$S/trees/sk-pristine" "$S/trees/sk-luaref"
for i in 1 2 3; do
  ( cd "$S/trees/sk-luaref" && lua buildSK.lua ) > "$S/logs/luaref-$i.log" 2>&1; echo "luaref run $i rc=$?"
  ( cd "$S/trees/sk-luaref" && $ID skbuilt.bin && ls sonic3k.log 2>/dev/null; cp skbuilt.bin "$S/logs/ref-run$i.bin" )
done
cp "$S/logs/ref-run3.bin" "$S/ref.bin"
cp -a "$S/trees/sk-pristine" "$S/trees/sk-gen"
for d in Sound/DAC/generated Sound/PCM/generated; do
  mkdir -p "$S/trees/sk-gen/$d"
  cp -a "$S/trees/sk-luaref/$d/." "$S/trees/sk-gen/$d/"
done
printf 'Sonic3_Complete = 0\n\tinclude "sonic3k.asm"\n' > "$S/trees/sk-gen/wrapper.asm"
echo "sk-gen: $(find "$S/trees/sk-gen" -type f | wc -l) files"
diff -rq "$S/trees/sk-pristine" "$S/trees/sk-gen" | sed 's|/[^/]*$||' | sort | uniq -c
echo "luaref extra files vs pristine:"
diff -rq "$S/trees/sk-pristine" "$S/trees/sk-luaref" | sed 's|/[^/]*$||' | sort | uniq -c
echo MKTREES_END
