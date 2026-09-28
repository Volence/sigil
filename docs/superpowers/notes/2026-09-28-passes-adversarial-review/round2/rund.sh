#!/bin/bash
# usage: rund.sh probe.asm DEF  -> run base/new with -D DEF and asl with -D DEF
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a86a73b35031b1394
P=$W/.probe
export AS_MSGPATH=$P/bin
f=$(realpath "$1"); n=$(basename "$f" .asm); o=$P/out/$n; mkdir -p "$o"; cp "$f" "$o/a.asm"
echo "== $n (-D $2)"
for v in base new; do
  "$W/.target-$v/release/sigil" "$f" -D "$2" -o "$o/$v.bin" > "$o/$v.log" 2>&1; rc=$?
  echo "$v rc=$rc $(xxd -p "$o/$v.bin" 2>/dev/null | tr -d '\n') $(grep -m1 error "$o/$v.log" | cut -c1-120)"
done
( cd "$o" && rm -f a.p a.bin && "$P/bin/asl" -xx -q -A -L -D "$2" a.asm > asl.log 2>&1; echo "asl rc=$?"; "$P/bin/p2bin" a.p a.bin >/dev/null 2>&1; xxd -p a.bin 2>/dev/null | tr -d '\n'; echo; grep -m1 error asl.log )
