#!/usr/bin/env bash
# ref_build.sh: the reference Sonic 2 ROM, built as build.lua builds it, with the
# REFERENCE asl (s1disasm's, md5 61e672562465725a8c102288a7da9098) in place of
# s2disasm's own (0dee1f98e6480a4783d27ffd8b90896f, refused by OVERSEER-REFERENCE).
#
# Stage A: swap the reference asl (+ its .msg files) into the copy's
#          build_tools/Linux-x86_64, then run build.lua UNMODIFIED. That is the
#          "exactly as build.lua does" ROM, but build.lua does not check asl's
#          exit status and deletes s2.p / s2.h.
# Stage B: re-run the ROM step by hand in the same tree through asl_run (exit
#          status + pass-loop completeness), keep s2.p / s2.h / s2.lst, keep
#          every intermediate image (p2bin raw, after amend, after fix_header),
#          and require the hand-built final image to equal Stage A's.
set -u
S=/home/volence/sonic_hacks/.scratch/s2-residual-census
REF=$S/ref
WT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a106286d7f094d71e
S1T=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64
OUT=$S/refout
mkdir -p "$OUT"
cd "$REF" || exit 9

echo "== stage A: swap reference asl into the copy"
for f in asl as.msg cmdarg.msg ioerrs.msg; do
  cp -f "$S1T/$f" "build_tools/Linux-x86_64/$f"
done
md5sum build_tools/Linux-x86_64/* | tee "$OUT/tool-md5s.txt"

echo "== stage A: lua build.lua (unmodified)"
lua build.lua > "$OUT/stageA-build.log" 2>&1
echo "LUA_BUILD_EXIT=$?" | tee -a "$OUT/stageA-build.log"
tail -3 "$OUT/stageA-build.log"
[ -f s2.log ] && { echo "s2.log PRESENT (asl wrote errors/warnings):"; cat s2.log; }
cp -f s2built.bin "$OUT/stageA-s2built.bin"
md5sum "$OUT/stageA-s2built.bin"

echo "== stage B: hand re-run of the ROM step through asl_run"
. "$WT/docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?
echo "guard ASL=$ASL md5=$ASL_REF_GOT"
rm -f s2.p s2.h s2.log
asl_run -xx -n -q -A -L -U -E -i . -c s2.asm
ASLRC=$?
echo "HAND_ASL_EXIT=$ASLRC" | tee "$OUT/stageB-asl-exit.txt"
[ -f s2.log ] && { echo "s2.log PRESENT:"; cat s2.log; }
[ "$ASLRC" -eq 0 ] || { echo "REFERENCE RUN FAILED; stopping stage B"; exit 4; }
cp -f s2.p "$OUT/s2.p"; cp -f s2.h "$OUT/s2.h"; cp -f s2.lst "$OUT/s2.lst"
P2BIN=build_tools/Linux-x86_64/p2bin
"$P2BIN" -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after s2.p "$OUT/b1-p2bin.bin" s2.h
echo "P2BIN_EXIT=$?"
cp -f "$OUT/b1-p2bin.bin" "$OUT/b2-amended.bin"
cp -f "$OUT/b2-amended.bin" "$OUT/b3-final.bin"
lua "$S/post_steps.lua" "$REF" "$OUT/s2.h" "$OUT/b2-amended.bin" "$OUT/b3-final.bin"
echo "POST_STEPS_EXIT=$?"
md5sum "$OUT"/*.bin
for b in "$OUT"/*.bin; do echo "$(basename "$b") size=$(stat -c%s "$b")"; done
if cmp -s "$OUT/stageA-s2built.bin" "$OUT/b3-final.bin"; then
  echo "STAGE_A_EQUALS_STAGE_B=yes"
else
  echo "STAGE_A_EQUALS_STAGE_B=NO"; cmp -l "$OUT/stageA-s2built.bin" "$OUT/b3-final.bin" | head
fi
echo REF_BUILD_END
