#!/usr/bin/env bash
# blindspot.sh: a positive control for what the whole-image byte compare CANNOT see.
# Plants a line no assembler could assemble inside a conditional arm the shipped
# switch settings exclude, then builds the tree with BOTH toolchains. If both still
# exit 0 and still write the reference ROM, the compare is demonstrated blind to
# every line in such an arm, and the demonstration has a control rather than an
# argument.
set -u
S=/home/volence/sonic_hacks/.scratch/as-corpus-census
plant() { # plant <tree> <file> <line> <marker>
  python3 - "$1/$2" "$3" "$4" <<'PY'
import sys
p, n, mark = sys.argv[1], int(sys.argv[2]), sys.argv[3]
L = open(p, encoding='latin-1').read().split('\n')
before = L[n-1]
L[n-1] = mark
open(p,'w',encoding='latin-1').write('\n'.join(L))
print('   planted at %s:%d' % (p.rsplit('/',1)[-1], n))
print('     was: %r' % before)
print('     now: %r' % mark)
PY
}
echo "== Sonic 2: s2.sounddriver.asm:2674 sits inside if OptimiseDriver (OptimiseDriver = 0)"
rm -rf $S/trees/s2disasm-blind; cp -a $S/trees/s2disasm-gen $S/trees/s2disasm-blind
plant $S/trees/s2disasm-blind s2.sounddriver.asm 2674 '	%%% THIS IS NOT AN INSTRUCTION %%% ,,, ((('
echo "   grep proof the mutation is on disk:"
grep -n 'THIS IS NOT AN INSTRUCTION' $S/trees/s2disasm-blind/s2.sounddriver.asm | sed 's/^/     /'
( cd $S/trees/s2disasm-blind && $S/target/release/sigil s2.asm -o blind.bin -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after ; echo "   SIGIL_EXIT=$?" )
echo "   sigil image: $(md5sum $S/trees/s2disasm-blind/blind.bin 2>/dev/null | cut -d' ' -f1)"
( cd $S/trees/s2disasm-blind && lua build.lua > /dev/null 2>&1 ; echo "   BUILDLUA_EXIT=$?" )
echo "   build.lua image: $(md5sum $S/trees/s2disasm-blind/s2built.bin 2>/dev/null | cut -d' ' -f1)"
echo "   reference:       9feeb724052c39982d432a7851c98d3e"
echo
echo "== CONTROL: the same marker on a LIVE line (s2.sounddriver.asm:2678) must be refused"
rm -rf $S/trees/s2disasm-blindctl; cp -a $S/trees/s2disasm-gen $S/trees/s2disasm-blindctl
plant $S/trees/s2disasm-blindctl s2.sounddriver.asm 2678 '	%%% THIS IS NOT AN INSTRUCTION %%% ,,, ((('
( cd $S/trees/s2disasm-blindctl && $S/target/release/sigil s2.asm -o blind.bin -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after 2>&1 | head -3 ; echo "   SIGIL_EXIT=${PIPESTATUS[0]}" )
echo BLINDSPOT_END

# --- CONTROL, second attempt. The first control line (2678) was itself inside the
# --- excluded arm (if OptimiseDriver spans 2657..2682), so it proved nothing. 1678
# --- is one of the ten live zAbsVar.1upPlaying sites the 2026-09-11 census names.
echo "== CONTROL (corrected): the same marker on a LIVE line, s2.sounddriver.asm:1678"
rm -rf $S/trees/s2disasm-blindctl2; cp -a $S/trees/s2disasm-gen $S/trees/s2disasm-blindctl2
python3 - $S/trees/s2disasm-blindctl2/s2.sounddriver.asm 1678 <<'PY'
import sys
p, n = sys.argv[1], int(sys.argv[2])
L = open(p, encoding='latin-1').read().split('\n')
print('   was: %r' % L[n-1])
L[n-1] = '\t%%% THIS IS NOT AN INSTRUCTION %%% ,,, ((('
open(p,'w',encoding='latin-1').write('\n'.join(L))
print('   now: %r' % L[n-1])
PY
grep -n 'THIS IS NOT AN INSTRUCTION' $S/trees/s2disasm-blindctl2/s2.sounddriver.asm | sed 's/^/     /'
( cd $S/trees/s2disasm-blindctl2 && $S/target/release/sigil s2.asm -o blind.bin -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after > /dev/null 2> ctl.err ; echo "   SIGIL_EXIT=$?" ; grep -v '^s2.asm(91275)' ctl.err | head -3 | sed 's/^/     /' )
( cd $S/trees/s2disasm-blindctl2 && lua build.lua > ctl.lua.log 2>&1 ; echo "   BUILDLUA_EXIT=$?" ; grep -iE 'error' ctl.lua.log | head -3 | sed 's/^/     /' )
echo CONTROL2_END
