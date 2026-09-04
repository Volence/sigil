#!/bin/bash
# Differential probe for AS's enum/nextenum/enumconf.
# usage: run.sh <file.asm>   -- runs S1's own asl with S1's own flags, prints the listing.
set -u
A=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl
F="$1"; B="${F%.asm}"
cd "$(dirname "$F")" || exit 9
rm -f "$B.lst" "$B.p"
"$A" -xx -n -q -A -L -U -i . "$(basename "$F")" 2>&1
echo "asl exit=$?"
grep -v '^ *$' "$B.lst" | sed -n '1,40p'
