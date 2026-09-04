#!/usr/bin/env bash
# Compare sigil's image against asl's for one probe file, by CRC32+size.
ASL=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl
P2BIN=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/p2bin
SIGIL=${SIGIL:-/home/volence/sonic_hacks/.sigil-struct/.target-land/release/sigil}
crc() { python3 -c "import zlib,sys;d=open(sys.argv[1],'rb').read();print('%08x/%d'%(zlib.crc32(d)&0xffffffff,len(d)))" "$1"; }
d=$(cd "$(dirname "$1")" && pwd); f=$(basename "$1" .asm)
cd "$d" || exit 2
rm -f "$f.p" "$f.aslbin" "$f.sigbin" "$f.log"
"$ASL" -xx -n -q -A -L -U -E -i . "$f.asm" >/dev/null 2>&1; ae=$?
[[ -f $f.p ]] && "$P2BIN" "$f.p" "$f.aslbin" >/dev/null 2>&1
"$SIGIL" "$f.asm" -o "$f.sigbin" >/dev/null 2>&1; se=$?
a=$( [[ -f $f.aslbin ]] && crc "$f.aslbin" || echo none )
s=$( [[ -f $f.sigbin ]] && crc "$f.sigbin" || echo none )
v=DIFFER; [[ $a == "$s" && $a != none ]] && v=SAME
printf '%-8s asl(exit=%d) %-20s sigil(exit=%d) %-20s %s\n' "$f" "$ae" "$a" "$se" "$s" "$v"
