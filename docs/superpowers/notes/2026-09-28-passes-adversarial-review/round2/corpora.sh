#!/bin/bash
# corpora.sh <sigil-binary> <tag>: build the author's extracted corpora (read only), outputs here
S=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-af82d7c20f212be15/.passes-scratch
O=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a86a73b35031b1394/.probe/corp
mkdir -p "$O"
BIN="$1"; TAG="$2"
for c in s1 s2 sk; do
  cd "$S/$c" || exit 2
  out="$O/$TAG-$c.bin"; rm -f "$out"
  case $c in
    s1) SIGIL_PHASE_TIMING=1 "$BIN" sonic.asm -o "$out" -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after > "$O/$TAG-$c.log" 2>&1; rc=$?; ref=s1built.bin;;
    s2) SIGIL_PHASE_TIMING=1 "$BIN" s2.asm -o "$out" -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after > "$O/$TAG-$c.log" 2>&1; rc=$?; ref=s2built.bin;;
    sk) SIGIL_PHASE_TIMING=1 "$BIN" sonic3k.asm -o "$out" -D Sonic3_Complete=0 -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before > "$O/$TAG-$c.log" 2>&1; rc=$?; ref=skbuilt.bin;;
  esac
  if [ -f "$out" ] && cmp -s "$out" "$ref"; then v=IDENTICAL; else v=DIFFERS; fi
  np=$(grep -cP '^SIGIL-PHASE\tpass[0-9]+\t' "$O/$TAG-$c.log")
  echo "$TAG $c rc=$rc passes=$np $v $(md5sum < "$ref" | cut -c1-8) $( [ -f "$out" ] && md5sum < "$out" | cut -c1-8)"
done
