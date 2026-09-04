#!/bin/bash
# Build one standalone .asm per S1 demo script, wrapping the corpus's OWN
# `demoinput` macro (copied verbatim out of Macros.asm) around the corpus's OWN
# demo data.  Assembling these with asl and with sigil and comparing the images
# byte-for-byte is the only way to see whether `irpc` expands to the RIGHT bytes:
# demo input data is emitted, never checked, so a wrong expansion is silent.
set -eu
S1=/home/volence/sonic_hacks/.s1-irpc
OUT=/home/volence/sonic_hacks/.sigil-irpc/.probe/demo
i=0
while IFS= read -r f; do
    i=$((i+1))
    n=$(printf 'demo%02d' "$i")
    {
        echo '	cpu 68000'
        echo '	padding off'
        sed -n '176,193p' "$S1/_Constants.asm"
        cat "$OUT/demoinput.inc"
        echo '	org $1000'
        cat "$S1/$f"; echo
        echo '	end'
    } > "$OUT/$n.asm"
    echo "$n	$f"
done < <(cd "$S1" && grep -rl demoinput --include='*.asm' demodata | sort)
