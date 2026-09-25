#!/usr/bin/env bash
# mk_trees.sh: pristine corpus copies (git archive tars), a luaref copy per build
# script where the stock lua build runs (the reference ROMs), and gen trees =
# pristine + the generated pre-step inputs (+ the S3K wrapper roots).
set -u
S=/home/volence/sonic_hacks/.scratch/as-squote
T=$S/trees
ID="python3 $S/scripts/ident.py"
echo "pwd=$(pwd) date=$(date -Is)"
for c in s1 s2 sk; do
  rm -rf "$T/$c-pristine"; mkdir -p "$T/$c-pristine"
  tar -x -C "$T/$c-pristine" -f "$S/$c.tar"
done
ref() { # ref <tag> <corpus> <script> <out> <runs>
  local tag="$1" c="$2" script="$3" out="$4" runs="$5"
  rm -rf "$T/luaref-$tag"; cp -a "$T/$c-pristine" "$T/luaref-$tag"
  for i in $(seq 1 "$runs"); do
    ( cd "$T/luaref-$tag" && lua "$script" ) > "$S/logs/luaref-$tag-$i.log" 2>&1
    echo "luaref $tag run $i rc=$?"
    $ID "$T/luaref-$tag/$out"
    cp "$T/luaref-$tag/$out" "$S/refs/$tag-run$i.bin"
  done
}
mkdir -p "$S/refs"
ref s1 s1 build.lua s1built.bin 1
ref s2 s2 build.lua s2built.bin 1
ref sk sk buildSK.lua skbuilt.bin 1
ref s3c sk buildS3Complete.lua sonic3k.bin 1
ref s3 sk buildS3.lua s3built.bin 3
gen() { # gen <corpus> <luaref tag> <dirs...>
  local c="$1" tag="$2"; shift 2
  rm -rf "$T/$c-gen"; cp -a "$T/$c-pristine" "$T/$c-gen"
  for d in "$@"; do
    mkdir -p "$T/$c-gen/$d"; cp -a "$T/luaref-$tag/$d/." "$T/$c-gen/$d/"
  done
  echo "$c-gen: $(find "$T/$c-gen" -type f | wc -l) files"
}
gen s1 s1 sound/dac/pcm/generated sound/dac/dpcm/generated
gen s2 s2 sound/music/generated sound/PCM/generated sound/DAC/generated
gen sk sk Sound/DAC/generated Sound/PCM/generated
printf 'Sonic3_Complete = 0\n\tinclude "sonic3k.asm"\n' > "$T/sk-gen/wrapper.asm"
printf 'Sonic3_Complete = 1\n\tinclude "sonic3k.asm"\n' > "$T/sk-gen/wrapper1.asm"
echo MKTREES_END
