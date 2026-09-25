#!/usr/bin/env bash
# mk_trees.sh: pristine skdisasm (the census archive, md5-checked) and a gen tree =
# pristine + the generated PCM/DAC inputs (from the codepage parcel's luaref copy) +
# the census's wrapper root.
set -eu
S=/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex
S0=/home/volence/sonic_hacks/.scratch/s3k-codepage
cp "$S0/sk.tar" "$S/sk.tar"
md5sum "$S/sk.tar"
rm -rf "$S/trees"
mkdir -p "$S/trees/sk-pristine"
tar -x -C "$S/trees/sk-pristine" -f "$S/sk.tar"
cp -a "$S/trees/sk-pristine" "$S/trees/sk-gen"
for d in Sound/DAC/generated Sound/PCM/generated; do
  mkdir -p "$S/trees/sk-gen/$d"
  cp -a "$S0/trees/sk-luaref/$d/." "$S/trees/sk-gen/$d/"
done
printf 'Sonic3_Complete = 0\n\tinclude "sonic3k.asm"\n' > "$S/trees/sk-gen/wrapper.asm"
if diff -rq "$S/trees/sk-gen" "$S0/trees/sk-gen"; then echo IDENTICAL_TO_CODEPAGE_GEN; fi
diff -rq "$S/trees/sk-pristine" "$S/trees/sk-gen" | sed 's|/[^/]*$||' | sort | uniq -c || true
echo MKTREES_END
