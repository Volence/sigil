#!/usr/bin/env bash
# stub_run2.sh: stub C (A + escape-free charset spellings) through the s2
# toolchain must still give the reference md5; stub D (C + placement) through
# sigil, then the same windowed compare as stub B. If the escape defect is the
# whole of stub B's diff, stub D's diff outside the window is zero.
set -u
S=/home/volence/sonic_hacks/.scratch/s2-residual-census
cd "$S" || exit 9
python3 stub.py "$S/corpus-gen" "$S/corpus-stubC" C || exit 5
python3 stub.py "$S/corpus-gen" "$S/corpus-stubD" D || exit 5
echo "== stub C through the s2 toolchain"
rm -rf "$S/asl-stubC"; cp -a "$S/corpus-stubC" "$S/asl-stubC"
cd "$S/asl-stubC" || exit 9
build_tools/Linux-x86_64/asl -xx -n -q -A -L -U -E -i . -c s2.asm > "$S/runs/asl-stubC.stdout" 2>&1
echo "STUBC_ASL_EXIT=$? log=$([ -f s2.log ] && echo PRESENT || echo absent)"
[ -f s2.log ] && head -20 s2.log
build_tools/Linux-x86_64/p2bin -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after s2.p s2built.bin s2.h
echo "STUBC_P2BIN_EXIT=$?"
echo "stubC asl ROM: $(md5sum s2built.bin | cut -d' ' -f1) size=$(stat -c%s s2built.bin)"
cmp -s s2built.bin "$S/ref2out/b3-final.bin" && echo "STUBC_BYTE_NEUTRAL=yes" || echo "STUBC_BYTE_NEUTRAL=NO"
cd "$S" || exit 9
bash run_sigil.sh "$S/corpus-stubD" "$S/runs/stubD"
python3 compare.py runs/stubD/s2.sigil.bin ref2out/b3-final.bin ref2out/s2.lst ref2out/z80blob.bin ref2out/s2.run1.p > runs/compare-stubD.txt 2>&1
echo "COMPARE_EXIT=$?"
/usr/bin/grep -E '^(WINDOW|CONTROL|image size|sigil bytes|DIFF|DRIVER|image bytes)' runs/compare-stubD.txt
echo STUB_RUN2_END
