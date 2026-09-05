#!/bin/bash
# Differential BYTE comparison: asl vs sigil on one source file.
# Usage: diff_bytes.sh <file.asm>
# Prints:  <name> asl=<crc32/size|ERR> sigil=<crc32/size|ERR> <SAME|DIFFER|...>
#
# THE DEFAULT ASSEMBLER IS SELECTED BY DIGEST, NOT BY BANNER — see
# `../asl-reference/`. Four `asl` binaries in this workspace print the same
# version string and are not the same program, so nothing in a listing said
# which had run. `|| exit $?` is load-bearing: `set -u` is not `set -e`.
#
# `ASL_CROSSCHECK_DIR` SURVIVES, because this directory's whole claim is that
# both shipped builds agree on every byte column here, and that agreement is a
# measurement to re-take rather than an assumption. The defect was never a
# second build — it was an UNIDENTIFIED instrument — so an explicit dir bypasses
# the digest pin and prints the md5 of whatever it was handed. It replaces the
# old `$ASL` override, which was a path with no digest attached.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
if [[ -n ${ASL_CROSSCHECK_DIR:-} ]]; then
    ASLDIR="$ASL_CROSSCHECK_DIR"
    ASL="$ASLDIR/asl"
    [[ -x $ASL ]] || { echo "FATAL: no executable asl at $ASL" >&2; exit 2; }
    echo "# CROSS-CHECK BUILD: $ASL md5 $(md5sum "$ASL" | cut -d' ' -f1)"
else
    . "$HERE/../asl-reference/asl_ref.sh" || exit $?
fi
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
