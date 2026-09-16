#!/usr/bin/env bash
# switchflip2.sh: as switchflip.sh but the edit is anchored to a LINE NUMBER, because
# the text form asserted "exactly one site" and sonic.asm:139 mentions `Revision = 1`
# inside a warning string. An assertion that fires is the only reason that run was not
# silently counted as a pass with an unapplied mutation.
set -u
S=/home/volence/sonic_hacks/.scratch/as-corpus-census
flipline() { # flipline <tag> <base> <root> <file> <line> <new text> <sigil args...>
  TAG="$1"; BASE="$2"; ROOT="$3"; FILE="$4"; LN="$5"; NEW="$6"; shift 6
  T=$S/trees/flip-$TAG
  rm -rf $T; cp -a $BASE $T
  python3 - "$T/$FILE" "$LN" "$NEW" <<'PY'
import sys
p, n, new = sys.argv[1], int(sys.argv[2]), sys.argv[3]
L = open(p, encoding='latin-1').read().split('\n')
print('   was: %r' % L[n-1]); L[n-1] = new; print('   now: %r' % L[n-1])
open(p,'w',encoding='latin-1').write('\n'.join(L))
PY
  echo "== $TAG : $FILE:$LN"
  sed -n "${LN}p" $T/$FILE | sed 's/^/   on disk: /'
  ( cd $T && $S/target/release/sigil $ROOT -o sigil.bin "$@" > /dev/null 2> sig.err; echo "   SIGIL_EXIT=$?"
    grep -v 'warning: .shared. is ignored' sig.err | head -3 | sed 's/^/     /' )
  ( cd $T && lua build.lua > lua.log 2>&1; echo "   BUILDLUA_EXIT=$?"
    grep -iE '^ *> >|error' lua.log | head -2 | sed 's/^/     /' )
  REF=$(/bin/ls $T/s1built.bin $T/s2built.bin 2>/dev/null | head -1)
  if [ -f "$T/sigil.bin" ] && [ -n "$REF" ]; then
    python3 $S/scripts/compare.py "$REF" "$T/sigil.bin" --control | sed 's/^/   /'
  else
    echo "   NOT COMPARABLE: sigil image $([ -f $T/sigil.bin ] && echo written || echo 'NOT written'), reference $([ -n "$REF" ] && echo written || echo 'NOT written')"
  fi
  echo
}
flipline s1-rev00   $S/trees/s1disasm-gen sonic.asm sonic.asm 14 'Revision = 0' -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after
flipline s1-cheats  $S/trees/s1disasm-gen sonic.asm sonic.asm 24 'CheatsEnabled = 1' -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after
flipline s1-sram    $S/trees/s1disasm-gen sonic.asm sonic.asm 37 'EnableSRAM = 1' -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after
flipline s2-water   $S/trees/s2disasm-gen s2.asm s2.asm 49 'useFullWaterTables = 1' -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after
flipline s2-nopad   $S/trees/s2disasm-gen s2.asm s2.asm 24 'padToPowerOfTwo = 0' -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after
echo SWITCHFLIP2_END
