#!/usr/bin/env bash
# Run one sigil binary over every AS corpus root, capturing stdout, stderr and exit.
# usage: run_roots.sh <binary> <tag>
set -u
BIN="$1"; TAG="$2"
S=/home/volence/sonic_hacks/.scratch/bonus-pass
C="$S/corpus"
OUT="$S/logs/$TAG"
mkdir -p "$OUT"

run_one() {
  name="$1"; dir="$2"; root="$3"
  ( cd "$dir" && timeout 900 "$BIN" "$root" ) > "$OUT/$name.out" 2> "$OUT/$name.err"
  echo "$?" > "$OUT/$name.exit"
  echo "$name exit=$(cat "$OUT/$name.exit") stdout=$(wc -l < "$OUT/$name.out") stderr=$(wc -l < "$OUT/$name.err")"
}

run_one s1            "$C/s1disasm"                   sonic.asm
run_one s2            "$C/s2disasm"                   s2.asm
run_one s2mompass     "$C/s2disasm-mompass-clean"     s2.asm
run_one sk            "$C/skdisasm"                   sonic3k.asm
run_one s3            "$C/skdisasm"                   s3.asm
run_one s4legacy      "$C/sonic_hack"                 S4.asm
run_one sce           "$C/Sonic-Clean-Engine-S.C.E.-" Engine/Includes.asm
run_one batman        "$C/batman"                     batman.asm
run_one mdos          "$C/MD-OS"                      Source.asm
echo "ALL-DONE-$TAG"
