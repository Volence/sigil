#!/usr/bin/env bash
# mk_gen_trees.sh: pristine corpus copy + the pre-step outputs build.lua generates
# before it assembles, taken from the luaref tree where the stock build.lua ran.
set -eu
S=/home/volence/sonic_hacks/.scratch/as-corpus-census
mk() { # mk <corpus> <generated dir>...
  c="$1"; shift
  rm -rf "$S/trees/$c-gen"
  cp -a "$S/trees/$c-pristine" "$S/trees/$c-gen"
  for d in "$@"; do
    mkdir -p "$S/trees/$c-gen/$d"
    cp -a "$S/trees/$c-luaref/$d/." "$S/trees/$c-gen/$d/"
  done
  echo "$c-gen: $(find $S/trees/$c-gen -type f | wc -l) files"
}
mk s1disasm sound/dac/pcm/generated sound/dac/dpcm/generated
mk s2disasm sound/music/generated sound/PCM/generated sound/DAC/generated
# the gen trees must not carry the reference build's OUTPUTS
for c in s1disasm s2disasm; do
  rm -f "$S/trees/$c-gen"/s1built.bin "$S/trees/$c-gen"/s2built.bin "$S/trees/$c-gen"/*.lst "$S/trees/$c-gen"/*.p "$S/trees/$c-gen"/*.h
done
echo MKGEN_END
