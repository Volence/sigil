#!/usr/bin/env bash
# probe.sh: what asl's -D does beyond the s3k-whole-rom dprobe table (asl_run only;
# bytes quoted from exit-0 runs only). Each case: label, probe file, then the asl
# arguments that precede the source (the -D spelling under test).
set -u
P=/home/volence/sonic_hacks/.scratch/as-cli-define/probes
NOTES=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a746dc1af1279ab3d/docs/superpowers/notes
. "$NOTES/asl-reference/asl_ref.sh" || exit $?
cd "$P" || exit 1
echo "pwd=$(pwd) asl=$ASL md5=$(md5sum "$ASL" | cut -d' ' -f1)"
run() {
  local label="$1" F="$2"; shift 2
  local B="${F%.asm}"
  rm -f "$B.p" "$B.bin" "$B.lst"
  echo "### $label: asl [$*] $F   ($(sed -n '2,$p' "$F" | tr '\n\t' '| '))"
  if asl_run -xx -n -q -A -L -U -i . "$@" "$F" 2>"$B.err"; then
    "$ASLDIR/p2bin" -p=0 "$B.p" "$B.bin" >/dev/null
    echo "  exit 0: $(xxd -p "$B.bin" | tr -d '\n')  [$(grep -hoE 'ASL_DIAG=[a-z]+' "$B.err")] passes: $(grep -hoE '^ +[0-9]+ passe?s?$' "$B.lst" | tr -d ' ')"
    grep -hE 'warning #' "$B.err" | head -3 | sed 's/^/    /'
  else
    echo "  refused: $(grep -hoE 'ASL_EXIT=[0-9]+' "$B.err")"
    grep -hE 'error #|warning #|rror' "$B.err" | grep -v 'REFUSED\|NO BYTE' | head -4 | sed 's/^/    /'
    grep -hvE '^ *$|ASL_|REFUSED|NO BYTE|lines that|reference is|listing prints|beside asl|Fix the source|^  ' "$B.err" | head -4 | sed 's/^/    other: /'
  fi
}
# spelling
run "attached -DFOO=5"             u1.asm -DFOO=5
run "attached -DFOO"               u1.asm -DFOO
run "repeated flag, distinct"      u2.asm -D FOO=1 -D BAR=2
run "repeated flag, same name"     u1.asm -D FOO=1 -D FOO=2
run "same name twice in one list"  u1.asm -D FOO=1,FOO=2
run "list, bare then valued"       u2.asm -D FOO,BAR=2
run "list with a space"            u2.asm -D 'FOO=1, BAR=2'
run "space before value"           u1.asm -D 'FOO= 5'
run "empty value"                  u1.asm -D FOO=
run "-D with nothing after it"     u1.asm -D
# values
run "value decimal"                u1.asm -D FOO=10
run "value lowercase hex"          u1.asm -D 'FOO=$ff'
run "value 0x hex"                 u1.asm -D FOO=0x10
run "value 10h"                    u1.asm -D FOO=10h
run "value %binary"                u1.asm -D FOO=%101
run "value negative"               u1.asm -D FOO=-1
run "value expression 1+2"         u1.asm -D FOO=1+2
run "value expression (3)"         u1.asm -D 'FOO=(3)'
run "value names a source symbol"  usebar.asm -D FOO=BAR
run "value double-quoted string"   strv.asm -D 'FOO="ab"'
run "value single-quoted char"     u1.asm -D "FOO='A'"
run "value float"                  u1.asm -D FOO=1.5
run "value \$FFFFFFFF, dc.l"       ul.asm -D 'FOO=$FFFFFFFF'
run "value \$100000000, dc.l"      ul.asm -D 'FOO=$100000000'
run "value \$7FFFFFFFFFFFFFFF, dc.l" ul.asm -D 'FOO=$7FFFFFFFFFFFFFFF'
run "value garbage"                u1.asm -D FOO=5x
# names
run "name starts with a digit"     u1.asm -D 1FOO=1
run "empty name"                   u1.asm -D =1
run "name with a dot"              u1.asm -D FOO.BAR=1
run "name with a dash"             u1.asm -D FO-O=1
run "name starts with a dot"       u1.asm -D .FOO=1
run "name starts with underscore"  u1.asm -D _FOO=1
run "lowercase -D, FOO in source"  casefoo.asm -D FOO=5
# interaction with the source
run "set first, then use"          setfirst.asm -D FOO=5
run "ifndef-guarded set"           guard.asm -D FOO=5
run "ifndef-guarded set, no -D"    guard.asm
run "ifdef on -D name"             ifdef.asm -D FOO=5
run "ifdef, -D FOO=0"              ifdef.asm -D FOO=0
run "ifndef on -D name"            ifndef.asm -D FOO=5
run "defined() on -D name"         defd.asm -D FOO=0
run "defined(), no -D"             defd.asm
run "label FOO: after -D"          label.asm -D FOO=5
run "FOO := 7 after -D"            colonset.asm -D FOO=5
run "FOO eval 7 after -D"          evalset.asm -D FOO=5
run "use, set, use, two passes"    mpass.asm -D FOO=5
echo PROBE_END
