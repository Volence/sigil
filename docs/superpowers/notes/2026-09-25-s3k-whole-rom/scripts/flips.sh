#!/usr/bin/env bash
# flips.sh: the two other build scripts skdisasm ships, each run unmodified in its own
# copy (twice, for stability), and sigil on the same inputs with the same p2bin
# instruction. S3 Complete needs a wrapper root (Sonic3_Complete = 1: sigil has no -D);
# S3 alone passes no -D and needs none.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
ID="python3 $S/scripts/ident.py"
mkdir -p "$S/flips"

# --- references ---
for spec in "s3c:buildS3Complete.lua:sonic3k.bin" "s3:buildS3.lua:s3built.bin"; do
  IFS=: read -r tag script out <<< "$spec"
  T=$S/trees/ref-$tag
  rm -rf "$T"; cp -a "$S/trees/sk-pristine" "$T"; rm -f "$T/wrapper.asm"
  for i in 1 2; do
    ( cd "$T" && lua "$script" ) > "$S/logs/ref-$tag-$i.log" 2>&1; echo "ref $tag run $i rc=$?"
    $ID "$T/$out"
    cp "$T/$out" "$S/flips/ref-$tag-run$i.bin"
  done
  ls "$T"/*.log 2>/dev/null && echo "asl log present for $tag" || echo "no asl .log for $tag"
done

# --- sigil ---
cp -a "$S/trees/sk-gen" "$S/trees/sig-flips" 2>/dev/null || true
rm -rf "$S/trees/sig-flips"; cp -a "$S/trees/sk-gen" "$S/trees/sig-flips"
printf 'Sonic3_Complete = 1\n\tinclude "sonic3k.asm"\n' > "$S/trees/sig-flips/wrapper1.asm"
bash "$S/scripts/run_sigil.sh" s3c "$S/trees/sig-flips" wrapper1.asm -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before
bash "$S/scripts/run_sigil.sh" s3 "$S/trees/sig-flips" s3.asm -p=FF -z=0,uncompressed,Size_of_Snd_driver_guess,before -z=1300,uncompressed,Size_of_Snd_driver2_guess,before

# --- fix_header applied to copies of sigil's images, by the toolchain's own lua ---
for tag in s3c s3; do
  cp "$S/runs/$tag/image.bin" "$S/trees/sig-flips/fixed-$tag.bin"
done
( cd "$S/trees/sig-flips" && lua -e 'local c = require "build_tools.lua.common"; c.fix_header("fixed-s3c.bin"); c.fix_header("fixed-s3.bin")' ); echo "fix_header rc=$?"
for tag in s3c s3; do
  cp "$S/trees/sig-flips/fixed-$tag.bin" "$S/flips/sigil-fixed-$tag.bin"
  cp "$S/runs/$tag/image.bin" "$S/flips/sigil-raw-$tag.bin"
  echo "== $tag: header fields, reference then raw sigil =="
  python3 "$S/scripts/header.py" "$S/flips/ref-$tag-run2.bin"
  python3 "$S/scripts/header.py" "$S/flips/sigil-raw-$tag.bin"
  echo "== $tag: reference vs RAW sigil =="
  python3 "$S/scripts/compare.py" "$S/flips/ref-$tag-run2.bin" "$S/flips/sigil-raw-$tag.bin"; echo "rc=$?"
  python3 "$S/scripts/diffranges.py" "$S/flips/ref-$tag-run2.bin" "$S/flips/sigil-raw-$tag.bin"
  echo "== $tag: reference vs sigil + fix_header =="
  python3 "$S/scripts/compare.py" "$S/flips/ref-$tag-run2.bin" "$S/flips/sigil-fixed-$tag.bin"; echo "rc=$?"
done
echo FLIPS_END
