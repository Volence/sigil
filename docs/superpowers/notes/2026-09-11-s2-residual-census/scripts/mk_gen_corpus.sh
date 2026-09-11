#!/usr/bin/env bash
# mk_gen_corpus.sh: a clean copy of s2disasm plus the build.lua PRE-STEP outputs
# (generate_music_data + WAV->PCM/DPCM conversion), taken from luaref where the
# stock build.lua ran. Also compares those generated files against the ones the
# reference asl produced in ref (stage A ran the same pre-steps with s1's asl).
set -u
S=/home/volence/sonic_hacks/.scratch/s2-residual-census
rm -rf "$S/corpus-gen"
cp -a /home/volence/sonic_hacks/s2disasm "$S/corpus-gen"
for d in sound/music/generated sound/PCM/generated sound/DAC/generated; do
  cp -a "$S/luaref/$d/." "$S/corpus-gen/$d/"
done
gen_md5() { (cd "$1" && find sound/music/generated sound/PCM/generated sound/DAC/generated -type f ! -name hashes.lua -print0 | sort -z | xargs -0 md5sum); }
gen_md5 "$S/luaref" > "$S/gen-md5.luaref.txt"
gen_md5 "$S/ref" > "$S/gen-md5.ref.txt"
gen_md5 "$S/corpus-gen" > "$S/gen-md5.corpus-gen.txt"
wc -l "$S"/gen-md5.*.txt
cmp -s "$S/gen-md5.luaref.txt" "$S/gen-md5.ref.txt" && echo "GENERATED s2-asl vs s1-asl: IDENTICAL" || { echo "GENERATED s2-asl vs s1-asl: DIFFER"; diff "$S/gen-md5.luaref.txt" "$S/gen-md5.ref.txt"; }
cmp -s "$S/gen-md5.luaref.txt" "$S/gen-md5.corpus-gen.txt" && echo "corpus-gen matches luaref" || echo "corpus-gen MISMATCH"
echo MK_END
