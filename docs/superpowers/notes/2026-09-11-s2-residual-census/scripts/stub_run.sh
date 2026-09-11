#!/usr/bin/env bash
# stub_run.sh: build stub A and stub B from corpus-gen, run sigil on both, and
# prove stub A byte-neutral by building it with the s2 toolchain (asl + p2bin +
# build.lua's post steps) and requiring the reference md5.
set -u
S=/home/volence/sonic_hacks/.scratch/s2-residual-census
cd "$S" || exit 9
python3 stub.py "$S/corpus-gen" "$S/corpus-stubA" A || exit 5
python3 stub.py "$S/corpus-gen" "$S/corpus-stubB" B || exit 5
bash run_sigil.sh "$S/corpus-stubA" "$S/runs/stubA"
bash run_sigil.sh "$S/corpus-stubB" "$S/runs/stubB"
echo "== stub A through the s2 toolchain (byte-neutrality control)"
cp -a "$S/corpus-stubA" "$S/asl-stubA"
cd "$S/asl-stubA" || exit 9
rm -f s2.p s2.h s2.log
build_tools/Linux-x86_64/asl -xx -n -q -A -L -U -E -i . -c s2.asm > "$S/runs/asl-stubA.stdout" 2>&1
rc=$?
echo "STUBA_ASL_EXIT=$rc log=$([ -f s2.log ] && echo PRESENT || echo absent)"
[ -f s2.log ] && head -20 s2.log
build_tools/Linux-x86_64/p2bin -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after s2.p s2built.bin s2.h
echo "STUBA_P2BIN_EXIT=$?"
lua "$S/post_steps.lua" "$S/asl-stubA" s2.h s2built.bin s2built.bin
echo "stubA asl ROM: $(md5sum s2built.bin | cut -d' ' -f1) size=$(stat -c%s s2built.bin)"
cmp -s s2built.bin "$S/ref2out/b3-final.bin" && echo "STUBA_BYTE_NEUTRAL=yes" || echo "STUBA_BYTE_NEUTRAL=NO"
echo STUB_RUN_END
