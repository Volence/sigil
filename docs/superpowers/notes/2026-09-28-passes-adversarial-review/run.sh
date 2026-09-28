#!/bin/bash
# usage: run.sh probe.asm...  -> per probe: base/new/asl rc, pass counts, image hex
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a86a73b35031b1394
P=$W/.probe
export AS_MSGPATH=$P/bin
for arg in "$@"; do
f=$(realpath "$arg"); n=$(basename "$f" .asm); o=$P/out/$n; mkdir -p "$o"
echo "== $n"
for v in base new; do
  bin=$W/.target-$v/release/sigil
  SIGIL_PHASE_TIMING=1 "$bin" "$f" -o "$o/$v.bin" > "$o/$v.log" 2>&1; rc=$?
  np=$(grep -cP '^SIGIL-PHASE\tpass[0-9]+\t' "$o/$v.log")
  oc=$(grep -oP '^SIGIL-PHASE\tpass[0-9]+\t.*?outcome=\K[a-z-]*' "$o/$v.log" | tail -1)
  by=$(grep -oP 'converged_by=\K[a-z]*' "$o/$v.log" | tail -1)
  if [ $rc -eq 0 ]; then hex=$(xxd -p "$o/$v.bin" | tr -d '\n'); else hex="ERR($(grep -v '^SIGIL-PHASE' "$o/$v.log" | grep -m1 -i 'error' | cut -c1-140))"; fi
  printf '%-5s rc=%d passes=%-2s %-28s %s\n' $v $rc "$np" "$oc/$by" "$hex"
done
cp "$f" "$o/a.asm"
( cd "$o" && rm -f a.p a.bin a.lst && "$P/bin/asl" -xx -q -A -L a.asm > asl.log 2>&1; echo $? > asl.rc )
rc=$(cat "$o/asl.rc")
ap=$(grep -ohP '[0-9]+ pass' "$o/a.lst" "$o/asl.log" 2>/dev/null | tail -1)
if [ -f "$o/a.p" ] && [ "$rc" = 0 ]; then ( cd "$o" && "$P/bin/p2bin" a.p a.bin >/dev/null 2>&1 ); hex=$(xxd -p "$o/a.bin" | tr -d '\n'); else hex="ERR($(grep -m1 -i 'error' "$o/asl.log" "$o/a.lst" 2>/dev/null | cut -c1-160))"; fi
printf '%-5s rc=%s %-37s %s\n' asl "$rc" "$ap" "$hex"
done
