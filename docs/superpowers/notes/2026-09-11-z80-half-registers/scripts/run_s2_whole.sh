#!/usr/bin/env bash
# run_s2_whole.sh <binary-name> : Sonic 2 whole, in the corpora/s2disasm copy
# (build.lua's pre-steps already run there), with build.lua's own p2bin
# instruction, compared byte for byte with build.lua's ROM from luaref.
set -u
S=/home/volence/sonic_hacks/.scratch/z80-half-registers
B=$S/bin/sigil-$1
T=$S/corpora/s2disasm
REF=$S/s2/luaref/s2built.bin
OUT=$S/runs/s2whole-$1
mkdir -p "$OUT"
rm -f "$OUT/s2sigil.bin"
echo "sigil $B md5 $(md5sum < "$B" | cut -c1-32)"
echo "command (cwd $T): $B s2.asm -o $OUT/s2sigil.bin -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after"
start=$(date +%s)
(cd "$T" && "$B" s2.asm -o "$OUT/s2sigil.bin" -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after > "$OUT/stdout" 2> "$OUT/stderr")
echo "SIGIL_EXIT=$? seconds=$(( $(date +%s) - start )) stderr-rows=$(wc -l < "$OUT/stderr")"
cat "$OUT/stderr"
echo "stdout:"; cat "$OUT/stdout"
python3 - "$OUT/s2sigil.bin" "$REF" <<'EOF'
import sys, zlib, hashlib, os
a, r = sys.argv[1], sys.argv[2]
ref = open(r, "rb").read()
print(f"REF   md5 {hashlib.md5(ref).hexdigest()} crc32 {zlib.crc32(ref):08x} size {len(ref)}")
if not os.path.exists(a):
    print("SIGIL wrote no image")
    sys.exit(0)
img = open(a, "rb").read()
print(f"SIGIL md5 {hashlib.md5(img).hexdigest()} crc32 {zlib.crc32(img):08x} size {len(img)}")
runs = []
for x in range(min(len(img), len(ref))):
    if img[x] != ref[x]:
        if runs and runs[-1][1] == x:
            runs[-1][1] = x + 1
        else:
            runs.append([x, x + 1])
print(f"DIFF {sum(b - a for a, b in runs)} bytes in {len(runs)} runs; sizes equal: {len(img) == len(ref)}")
for a0, b0 in runs[:40]:
    print(f"  [{a0:#08x},{b0:#08x}) sigil {img[a0:b0][:16].hex(' ')} ref {ref[a0:b0][:16].hex(' ')}")
EOF
echo S2WHOLE_END
