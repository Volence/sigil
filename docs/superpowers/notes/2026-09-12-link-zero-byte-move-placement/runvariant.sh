#!/usr/bin/env bash
# usage: runvariant.sh <variant> <constants: cae|parent> [game] [debug] [fast]
# Installs boot_data.<variant>.emp and the chosen constants.emp into the aeon repro tree,
# builds one shape, and prints rc, ROM CRC32+size and any overlap line.
set -u
S=/home/volence/sonic_hacks/.scratch/link-zero-byte-move
T=/home/volence/sonic_hacks/.sigil-zbm-aeon-cae58661
v=$1; c=$2; game=${3:-sonic4}; dbg=${4:-1}; fast=${5:-1}
cp "$S/boot_data.$v.emp" "$T/engine/system/boot_data.emp" || exit 9
if [[ $c == parent ]]; then
  cp "$S/constants.cae58661-parent.emp" "$T/engine/system/constants.emp"
else
  cp "$S/constants.cae58661.emp" "$T/engine/system/constants.emp"
fi
out=$("$S/shape.sh" "var-$v-$c" "$game" "$dbg" "$fast")
echo "$out"
log=$S/runs/var-$v-$c.$game.d$dbg.f$fast.log
grep -o 'sections `[^`]*` \[[^]]*) and `[^`]*` \[[^]]*) overlap[^"]*' "$log" | head -2
grep -m2 '^error' "$log" | cut -c1-300
