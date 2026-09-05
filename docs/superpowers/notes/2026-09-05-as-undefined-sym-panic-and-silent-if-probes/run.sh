#!/bin/bash
SIGIL=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a159ca0b948688d11/.target-land/release/sigil
cd /tmp/claude-1000/-home-volence-sonic-hacks-sigil/1a93ba92-b503-43b3-8939-b5973f7954ac/scratchpad/p
for f in "$@"; do
  echo "=== $f ==="
  "$SIGIL" "$f.asm" --hex 2>&1
  echo "EXIT=$?"
done
