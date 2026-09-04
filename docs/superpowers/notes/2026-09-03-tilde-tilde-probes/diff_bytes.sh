#!/bin/bash
# Differential BYTE comparison: asl vs sigil on one source file.
# Usage: diff_bytes.sh <file.asm>
# Prints:  <name> asl=<crc32/size|ERR> sigil=<crc32/size|ERR> <SAME|DIFFER|...>
set -u
ASL=${ASL:-/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl}
P2BIN=${P2BIN:-/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/p2bin}
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
