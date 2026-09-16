#!/usr/bin/env bash
# switchflip.sh: FEASIBILITY probe for converting the compare's blind region into
# measured region. Flips one of a corpus's own assembly-option switches, builds the
# tree with BOTH toolchains, and compares the two images to each other. No reference
# ROM exists for a flipped tree, so the question is not "is it the retail ROM" but
# "do sigil and asl+p2bin agree on source neither has been compared on".
set -u
S=/home/volence/sonic_hacks/.scratch/as-corpus-census
flip() { # flip <tag> <base tree> <root> <file> <old line> <new line> <sigil args...>
  TAG="$1"; BASE="$2"; ROOT="$3"; FILE="$4"; OLD="$5"; NEW="$6"; shift 6
  T=$S/trees/flip-$TAG
  rm -rf $T; cp -a $BASE $T
  python3 - "$T/$FILE" "$OLD" "$NEW" <<'PY'
import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
t = open(p, encoding='latin-1').read()
assert t.count(old) == 1, ('site count', p, old, t.count(old))
open(p,'w',encoding='latin-1').write(t.replace(old, new))
PY
  echo "== $TAG : $FILE   $OLD  ->  $NEW"
  grep -n "^$NEW" $T/$FILE | head -1 | sed 's/^/   on disk: /'
  ( cd $T && $S/target/release/sigil $ROOT -o sigil.bin "$@" > /dev/null 2> sig.err; echo "   SIGIL_EXIT=$?"
    [ -s sig.err ] && grep -v 'warning: .shared. is ignored' sig.err | head -3 | sed 's/^/     /' )
  ( cd $T && lua build.lua > lua.log 2>&1; echo "   BUILDLUA_EXIT=$?"
    grep -iE '^ *> >|error' lua.log | head -3 | sed 's/^/     /' )
  REF=$(/bin/ls $T/s1built.bin $T/s2built.bin 2>/dev/null | head -1)
  if [ -f "$T/sigil.bin" ] && [ -n "$REF" ]; then
    python3 $S/scripts/compare.py "$REF" "$T/sigil.bin" --control | sed 's/^/   /'
  else
    echo "   NOT COMPARABLE: sigil.bin=$([ -f $T/sigil.bin ] && echo yes || echo no) reference=$([ -n "$REF" ] && echo yes || echo no)"
  fi
  echo
}
flip s2-rev00 $S/trees/s2disasm-gen s2.asm s2.asm 'gameRevision = 1' 'gameRevision = 0' -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after
flip s2-fixbugs $S/trees/s2disasm-gen s2.asm s2.asm 'fixBugs = 0' 'fixBugs = 1' -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after
flip s2-allopt $S/trees/s2disasm-gen s2.asm s2.asm 'allOptimizations = 0' 'allOptimizations = 1' -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after
flip s1-fixbugs $S/trees/s1disasm-gen sonic.asm sonic.asm 'FixBugs = 0' 'FixBugs = 1' -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after
flip s1-allopt $S/trees/s1disasm-gen sonic.asm sonic.asm 'AllOptimizations = 0' 'AllOptimizations = 1' -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after
flip s1-rev00 $S/trees/s1disasm-gen sonic.asm sonic.asm 'Revision = 1' 'Revision = 0' -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after
echo SWITCHFLIP_END
