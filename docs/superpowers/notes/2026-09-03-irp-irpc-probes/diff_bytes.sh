#!/bin/bash
# Differential BYTE comparison: asl vs sigil on one source file.
# Usage: diff_bytes.sh <file.asm>
# Prints:  <name> asl=<crc32/size|ERR> sigil=<crc32/size|ERR> <SAME|DIFFER|...>
#
# The assembler is selected by MD5, not by path and not by version banner: four
# `asl` binaries in this workspace print the same banner and are not the same
# program, and one of them answers refused operands from uninitialized memory.
# `asl_ref.sh` refuses anything but the reference build; `|| exit $?` is
# load-bearing because `set -u` is not `set -e`. There is deliberately no `$ASL`
# override any more — a differential runner that can be pointed at the varying
# build reports a difference the second build invented.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
P2BIN=${P2BIN:-$ASLDIR/p2bin}
SIG=${SIG:-/home/volence/sonic_hacks/.sigil-irpc/.target-land/release/sigil}
f=$1; b=${f%.asm}
rm -f "$b.p" "$b.log" "$b.lst" "$b.asl.bin" "$b.sig.bin"
AS_MSGPATH=$(dirname "$ASL") "$ASL" -xx -n -q -A -L -U -E -i . "$f" >/dev/null 2>&1
ax=$?
if [[ -f $b.p ]]; then "$P2BIN" "$b.p" "$b.asl.bin" -r '$-$' >/dev/null 2>&1; fi
"$SIG" "$f" -o "$b.sig.bin" >/dev/null 2>"$b.sig.err"
sx=$?
sum() { if [[ -f $1 ]]; then printf '%s/%s' "$(crc32 "$1" 2>/dev/null || python3 -c "import zlib,sys;print('%08x'%(zlib.crc32(open(sys.argv[1],'rb').read())&0xffffffff))" "$1")" "$(stat -c%s "$1")"; else printf 'ERR'; fi; }
a=$(sum "$b.asl.bin"); s=$(sum "$b.sig.bin")
if [[ $a == ERR || $s == ERR ]]; then v="UNCOMPARABLE(asl_exit=$ax sigil_exit=$sx)"
elif [[ $a == "$s" ]]; then v=SAME; else v=DIFFER; fi
printf '%-14s asl=%-18s sigil=%-18s %s\n' "$(basename "$b")" "$a" "$s" "$v"
