#!/usr/bin/env bash
# ref2_build.sh: the s2disasm toolchain's own asl (md5 0dee1f98..., the build
# OVERSEER-REFERENCE refuses for DECLINED operands), run by hand in the luaref
# copy where build.lua already ran clean. Three runs, each with its exit status,
# its listing's pass-loop footer, and the md5 of its object file: the varying
# build's defect shows up as run-to-run differences, so three identical object
# files under normal ASLR are the control that no declined operand reached a byte.
set -u
S=/home/volence/sonic_hacks/.scratch/s2-residual-census
L=$S/luaref
OUT=$S/ref2out
mkdir -p "$OUT"
cd "$L" || exit 9
echo "AS_MSGPATH=${AS_MSGPATH:-<unset>}"
echo "ASLR: $(cat /proc/sys/kernel/randomize_va_space)"
md5sum build_tools/Linux-x86_64/* | tee "$OUT/tool-md5s.txt"
ASL=build_tools/Linux-x86_64/asl
footer() {
  if /usr/bin/grep -qE '^[[:space:]]+Additional necessary passes not started' "$1"; then echo INCOMPLETE
  elif /usr/bin/grep -qE '^ +[0-9]+ passe?s?$' "$1"; then echo "complete ($(/usr/bin/grep -E '^ +[0-9]+ passe?s?$' "$1" | tr -s ' '))"
  else echo nofooter; fi
}
for n in 1 2 3; do
  rm -f s2.p s2.h s2.log
  "$ASL" -xx -n -q -A -L -U -E -i . -c s2.asm > "$OUT/run$n.stdout" 2>&1
  rc=$?
  echo "run$n ASL_EXIT=$rc footer=$(footer s2.lst) log=$([ -f s2.log ] && echo PRESENT || echo absent) p=$(md5sum s2.p | cut -d' ' -f1) h=$(md5sum s2.h | cut -d' ' -f1)"
  cp -f s2.p "$OUT/s2.run$n.p"
  [ "$n" = 1 ] && { cp -f s2.h "$OUT/s2.h"; cp -f s2.lst "$OUT/s2.lst"; }
done
P2BIN=build_tools/Linux-x86_64/p2bin
cp -f "$OUT/s2.h" "$OUT/s2.h.asl"
"$P2BIN" -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after "$OUT/s2.run1.p" "$OUT/b1-p2bin.bin" "$OUT/s2.h" > "$OUT/p2bin.stdout" 2>&1
echo "P2BIN_EXIT=$?"; cat "$OUT/p2bin.stdout"
cp -f "$OUT/b1-p2bin.bin" "$OUT/b2-amended.bin"
cp -f "$OUT/b1-p2bin.bin" "$OUT/b3-final.bin"
lua "$S/post_steps.lua" "$L" "$OUT/s2.h" "$OUT/b2-amended.bin" "$OUT/b3-final.bin"
echo "POST_STEPS_EXIT=$?"
for b in "$OUT"/*.bin "$L/s2built.bin"; do echo "$(md5sum "$b" | cut -d' ' -f1) size=$(stat -c%s "$b") crc32=$(python3 -c 'import sys,zlib;print("%08x"%(zlib.crc32(open(sys.argv[1],"rb").read())&0xffffffff))' "$b") $b"; done
cmp -s "$OUT/b3-final.bin" "$L/s2built.bin" && echo "HAND_FINAL_EQUALS_BUILD_LUA=yes" || echo "HAND_FINAL_EQUALS_BUILD_LUA=NO"
echo REF2_END
