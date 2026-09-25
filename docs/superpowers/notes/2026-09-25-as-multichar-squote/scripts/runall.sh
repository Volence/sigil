#!/usr/bin/env bash
# runall.sh <listfile> <logfile>: probe.sh over every name in listfile.
S=/home/volence/sonic_hacks/.scratch/as-squote
{
  echo "pwd=$(pwd) base=$("$S/target-base/release/sigil" --version | sed -n 2p)"
  [ -x "$S/target/release/sigil" ] && echo "tip=$("$S/target/release/sigil" --version | sed -n 2,5p | tr '\n' ' ')"
  for n in $(cat "$1"); do bash "$S/scripts/probe.sh" "$n.asm"; done
  echo RUNALL_END
} > "$2" 2>&1
