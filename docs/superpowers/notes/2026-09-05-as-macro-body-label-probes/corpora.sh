#!/usr/bin/env bash
# The corpus half of the census: assemble each disassembly with ONE sigil binary
# and dump its whole diagnostic stream. Run it twice — branch-point binary, then
# the parcel's — and diff the two dumps. A grep cannot enumerate this population
# (the names come out of macro expansion and `\{}` interpolation, so they are not
# in the source text); the compiler's own stream is the enumeration.
#
#   ./corpora.sh <sigil-binary> <out-dir>
set -uo pipefail
SIGIL="$1"; OUT="$2"
mkdir -p "$OUT"
for c in "s1:/home/volence/sonic_hacks/s1disasm:sonic.asm" \
         "s2:/home/volence/sonic_hacks/s2disasm:s2.asm"; do
    n="${c%%:*}"; rest="${c#*:}"; dir="${rest%%:*}"; root="${rest#*:}"
    ( cd "$dir" && "$SIGIL" "$root" >/dev/null 2>"$OUT/$n.diag" )
    echo "$n SIGIL_EXIT=$? diagnostics=$(wc -l < "$OUT/$n.diag")"
done
