#!/bin/bash
# Differential probe: run asl (S1's own binary, S1's own flags) and sigil on the
# same source, print both exit codes and both byte outputs.
# usage: cmp.sh <file.asm>
set -u
F="$1"
B="${F%.asm}"
D="$(cd "$(dirname "$F")" && pwd)"
cd "$D" || exit 9
ASL=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl
P2BIN=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/p2bin
SIGIL=/home/volence/sonic_hacks/.sigil-s1recon-target/release/sigil
rm -f "$B.p" "$B.lst" "$B.log" "$B.asl.bin" "$B.sigil.bin"
"$ASL" -xx -n -q -A -L -U -i . "$(basename "$F")" > "$B.asl.out" 2>&1
echo "asl exit=$?"
sed -n '1,200p' "$B.asl.out"
if [ -f "$B.p" ]; then
  "$P2BIN" -p=0 "$B.p" "$B.asl.bin" >/dev/null 2>&1
  echo "asl bytes: $(od -An -tx1 -v "$B.asl.bin" | tr -s ' ' | tr -d '\n' | sed 's/^ //')"
else
  echo "asl bytes: (no .p produced)"
fi
echo "--- asl listing:"
if [ -f "$B.lst" ]; then grep -v '^ *$' "$B.lst" | sed -n '1,60p'; fi
echo "--- sigil:"
"$SIGIL" "$(basename "$F")" --hex 2>&1 | sed -n '1,60p'
echo "sigil exit=$?"
