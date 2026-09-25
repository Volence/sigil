#!/usr/bin/env bash
# flip_kosopt.sh: buildSK.lua with its own setting improved_sound_driver_compression = true
# (p2bin -z ...,kosinski-optimised,...), edit shown on disk, twice; sigil with the same -z.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
T=$S/trees/ref-kosopt
rm -rf "$T"; cp -a "$S/trees/sk-pristine" "$T"; rm -f "$T/wrapper.asm"
sed -i 's/^local improved_sound_driver_compression = false$/local improved_sound_driver_compression = true/' "$T/buildSK.lua"
diff "$S/trees/sk-pristine/buildSK.lua" "$T/buildSK.lua" && { echo "EDIT NOT APPLIED"; exit 1; }
for i in 1 2; do
  ( cd "$T" && lua buildSK.lua ) > "$S/logs/ref-kosopt-$i.log" 2>&1; echo "ref run $i rc=$?"
  python3 "$S/scripts/ident.py" "$T/skbuilt.bin"
done
mkdir -p "$S/flips"; cp "$T/skbuilt.bin" "$S/flips/ref-kosopt.bin"
bash "$S/scripts/run_sigil.sh" kosopt "$S/trees/sk-gen" wrapper.asm -p=FF -z=0,kosinski-optimised,Size_of_Snd_driver_guess,before -z=1300,kosinski-optimised,Size_of_Snd_driver2_guess,before
[ -f "$S/runs/kosopt/image.bin" ] && {
  python3 "$S/scripts/compare.py" "$S/flips/ref-kosopt.bin" "$S/runs/kosopt/image.bin"; echo "compare rc=$?"
  python3 "$S/scripts/diffranges.py" "$S/flips/ref-kosopt.bin" "$S/runs/kosopt/image.bin"
  python3 "$S/scripts/diffranges.py" "$S/ref.bin" "$S/flips/ref-kosopt.bin" | head -3
}
echo KOSOPT_END
