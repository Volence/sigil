#!/usr/bin/env bash
# parity.sh: every -D probe run through asl (asl_run only) and through one sigil
# binary, one row each: `asl <bytes|refused rc>` and `sigil <bytes|refused rc>`,
# then MATCH or DIFFER. A refusal matches a refusal; exit-0 bytes must be equal.
# Usage: parity.sh <sigil-binary>
set -u
SIGIL="$1"
P=/home/volence/sonic_hacks/.scratch/as-cli-define/probes
NOTES=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a746dc1af1279ab3d/docs/superpowers/notes
. "$NOTES/asl-reference/asl_ref.sh" || exit $?
cd "$P" || exit 1
echo "pwd=$(pwd) asl_md5=$(md5sum "$ASL" | cut -d' ' -f1) sigil=$SIGIL"
"$SIGIL" --version | sed -n '1,4p'
n=0; match=0
row() {
  local label="$1" F="$2"; shift 2
  local B="${F%.asm}" a s
  rm -f "$B.p" "$B.bin" "$B.lst" "$B.sig.bin"
  if asl_run -xx -n -q -A -L -U -i . "$@" "$F" 2>"$B.err" >/dev/null; then
    "$ASLDIR/p2bin" -p=0 "$B.p" "$B.bin" >/dev/null
    a="bytes $(xxd -p "$B.bin" | tr -d '\n')"
  else
    a="refused rc=$(grep -hoE 'ASL_EXIT=[0-9]+' "$B.err" | cut -d= -f2)"
  fi
  if "$SIGIL" "$F" -o "$B.sig.bin" "$@" >"$B.sig.out" 2>"$B.sig.err"; then
    s="bytes $(xxd -p "$B.sig.bin" | tr -d '\n')"
  else
    s="refused rc=$? ($(grep -m1 -E 'error' "$B.sig.err"))"
  fi
  n=$((n+1))
  local verdict=DIFFER
  if [ "${a%% *}" = refused ] && [ "${s%% *}" = refused ]; then verdict=MATCH; fi
  if [ "${a%% *}" = bytes ] && [ "$a" = "$s" ]; then verdict=MATCH; fi
  [ $verdict = MATCH ] && match=$((match+1))
  printf "%s | %s | args: %s | asl: %s | sigil: %s\n" "$verdict" "$label" "$*" "$a" "$s"
}
# the s3k-whole-rom dprobe table
row "value given"                d1.asm -D FOO=5
row "no value"                   d1.asm -D FOO
row "comma list"                 d2.asm -D FOO=1,BAR=2
row "hex value"                  d1.asm -D 'FOO=$10'
row "in-file = after -D"         d3.asm -D FOO=5
row "case: -D FOO, use foo"      d4.asm -D FOO=5
row "if on -D, 0"                d5.asm -D FOO=0
row "if on -D, 1"                d5.asm -D FOO=1
row "in-file set after -D"       d6.asm -D FOO=5
row "in-file equ after -D"       d7.asm -D FOO=5
row "use, set, use"              d8.asm -D FOO=5
# this parcel's probes
row "repeated flag, distinct"    u2.asm -D FOO=1 -D BAR=2
row "repeated flag, same name"   u1.asm -D FOO=1 -D FOO=2
row "same name twice in a list"  u1.asm -D FOO=1,FOO=2
row "bare then valued"           u1.asm -D FOO -D FOO=2
row "valued then bare"           u1.asm -D FOO=5 -D FOO
row "list, bare then valued"     u2.asm -D FOO,BAR=2
row "list of three"              u2.asm -D FOO=1,BAR=2,BAZ=3
row "trailing comma"             u1.asm -D FOO=5,
row "empty value"                u1.asm -D FOO=
row "space before value"         u1.asm -D 'FOO= 5'
row "trailing space in value"    u1.asm -D 'FOO=5 '
row "value decimal"              u1.asm -D FOO=10
row "value lowercase hex"        u1.asm -D 'FOO=$ff'
row "value 0x hex"               u1.asm -D FOO=0x10
row "value 10h"                  u1.asm -D FOO=10h
row "value 0FFh"                 u1.asm -D FOO=0FFh
row "value %binary"              u1.asm -D FOO=%101
row "value negative"             u1.asm -D FOO=-1
row "value ~1"                   u1.asm -D 'FOO=~1'
row "value 1+2"                  u1.asm -D FOO=1+2
row "value 1 + 2"                u1.asm -D 'FOO=1 + 2'
row "value 1<<4"                 u1.asm -D 'FOO=1<<4'
row "value (3)"                  u1.asm -D 'FOO=(3)'
row "value 1=1"                  u1.asm -D 'FOO=1=1'
row "value \$FFFFFFFF, dc.l"     ul.asm -D 'FOO=$FFFFFFFF'
row "value \$100000000, dc.l"    ul.asm -D 'FOO=$100000000'
row "value \$123456789, >>32"    uhi.asm -D 'FOO=$123456789'
row "list with a space"          u2.asm -D 'FOO=1, BAR=2'
row "empty middle part"          u2.asm -D FOO=1,,BAR=2
row "leading comma"              u1.asm -D ,FOO=5
row "space before ="             u1.asm -D 'FOO =5'
row "empty argument"             u1.asm -D ''
row "double equals"              u1.asm -D FOO==5
row "bare dollar"                u1.asm -D 'FOO=$'
row "value 5x"                   u1.asm -D FOO=5x
row "value 1+"                   u1.asm -D 'FOO=1+'
row "value ff"                   u1.asm -D FOO=ff
row "value names a symbol"       usebar.asm -D FOO=BAR
row "value float, dc.b"          u1.asm -D FOO=1.5
row "name starts with a digit"   u1.asm -D 1FOO=1
row "empty name"                 u1.asm -D =1
row "name with a dash"           u1.asm -D FO-O=1
row "name FOO?"                  uq.asm -D 'FOO?=1'
row "name @FOO"                  uat.asm -D @FOO=1
row "name with a dot, used"      udot.asm -D FOO.BAR=1
row "underscore name, used"      uus.asm -D _FOO=1
row "leading-dot name, .FOO used" uldot.asm -D .FOO=1
row "lowercase -D, FOO used"     u1.asm -D foo=5
row "-D FOO, foo equ 3 coexist"  casefoo.asm -D FOO=5
row "set first, then use"        setfirst.asm -D FOO=5
row "ifndef-guarded set"         guard.asm -D FOO=5
row "ifndef-guarded set, no -D"  guard.asm
row "ifdef on -D name"           ifdef.asm -D FOO=5
row "ifdef, -D FOO=0"            ifdef.asm -D FOO=0
row "ifndef on -D name"          ifndef.asm -D FOO=5
row "defined() on -D name"       defd.asm -D FOO=0
row "defined(), no -D"           defd.asm
row "label FOO: after -D"        label.asm -D FOO=5
row "FOO := 7 after -D"          colonset.asm -D FOO=5
row "FOO eval 7 after -D"        evalset.asm -D FOO=5
row "use, set, use, 2 passes"    mpass.asm -D FOO=5
row "-D MOMCPU (a builtin)"      umom.asm -D MOMCPU=5
# known divergences: asl accepts, sigil refuses at the command line (expected DIFFER)
row "KNOWN: string value"         strA.asm -D 'FOO="A"'
row "KNOWN: char value"           u1.asm -D "FOO='A'"
row "KNOWN: value 0b101"          u1.asm -D FOO=0b101
row "KNOWN: value @17"            u1.asm -D FOO=@17
row "KNOWN: value \$FFFFFFFFFFFFFFFF" uhi.asm -D 'FOO=$FFFFFFFFFFFFFFFF'
row "KNOWN: leading-dot name, unused" d1.asm -D .FOO=1 -D FOO=2
echo "PARITY_END rows=$n match=$match differ=$((n-match))"
