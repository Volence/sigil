#!/usr/bin/env bash
# The corpus half of the landing condition: assemble each disassembly with ONE
# sigil binary and record its image CRC32+size and its whole diagnostic stream.
# Run it twice — branch-point binary, then the parcel's — and compare.
#
#   ./corpora.sh <sigil-binary> <out-dir>
#
# The CRC is the load-bearing half. A diff of the diagnostic streams alone cannot
# tell "the rule engaged and agreed" from "the rule never ran"; that is what
# census.sh answers, and the two are meant to be read together.
set -uo pipefail
SIGIL="$1"; OUT="$2"
mkdir -p "$OUT"
for c in "s1:/home/volence/sonic_hacks/s1disasm:sonic.asm" \
         "s2:/home/volence/sonic_hacks/s2disasm:s2.asm"; do
    n="${c%%:*}"; rest="${c#*:}"; dir="${rest%%:*}"; root="${rest#*:}"
    ( cd "$dir" && "$SIGIL" "$root" -o "$OUT/$n.bin" >"$OUT/$n.out" 2>"$OUT/$n.diag" )
    rc=$?
    if [[ -f $OUT/$n.bin ]]; then
        printf '%s\tSIGIL_EXIT=%d\tcrc32=%s\tsize=%s\tdiagnostics=%s\n' \
            "$n" "$rc" \
            "$(python3 -c 'import sys,zlib;print("%08x"%(zlib.crc32(open(sys.argv[1],"rb").read())&0xffffffff))' "$OUT/$n.bin")" \
            "$(stat -c%s "$OUT/$n.bin")" \
            "$(wc -l < "$OUT/$n.diag")"
    else
        echo "$n SIGIL_EXIT=$rc (NO IMAGE) diagnostics=$(wc -l < "$OUT/$n.diag")"
        head -5 "$OUT/$n.diag"
    fi
done
