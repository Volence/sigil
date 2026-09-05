#!/bin/bash
SIGIL=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f/.target-land/release/sigil
cd /home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f/.mompassprobe
for f in pA pB pE pG pH pI; do
  sed 's/MOMPASS/0/g' $f.asm > $f.nom.asm
  err=$(SIGIL_PASS_TRACE=1 "$SIGIL" $f.nom.asm --hex 2>&1 >/dev/null)
  n=$(printf '%s\n' "$err" | grep -c '^SIGIL-ITER')
  out=$(SIGIL_PASS_TRACE=1 "$SIGIL" $f.nom.asm --hex 2>/dev/null); rc=$?
  echo "$f: sigil_iterations=$n exit=$rc"
done
echo ITERS-END
