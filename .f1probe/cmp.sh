#!/bin/bash
# Differential probe: run the reference asl (S1's own flags) and sigil on the
# same source, print both exit codes and both byte outputs.
# usage: cmp.sh <file.asm>
#
# The assembler is selected by MD5, not by path and not by version banner: four
# `asl` binaries in this workspace print `Macro Assembler 1.42 Beta [Bld 212]`
# verbatim and are not the same program, and one of them answers any operand it
# declined to value from uninitialized memory. `asl_ref.sh` refuses anything but
# the reference build. `|| exit $?` is load-bearing — `set -u` is not `set -e`.
# HERE is resolved BEFORE the `cd`, so the guard is found from this file's own
# directory rather than from the probe source's.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/../docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?
F="$1"
B="${F%.asm}"
D="$(cd "$(dirname "$F")" && pwd)"
cd "$D" || exit 9
P2BIN="$ASLDIR/p2bin"
SIGIL=${SIGIL:-/home/volence/sonic_hacks/.sigil-f1-target/release/sigil}
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
