#!/usr/bin/env bash
# runprobes.sh <sigil-binary> <probe-name>... : run each committed evidence
# probe through the given sigil binary and print its first two output lines,
# beside asl's recorded exit status.
SIGIL="$1"; shift
P=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a09c77cfcd76b4fb6/docs/superpowers/notes/2026-09-12-as-missing-builtins/probes
echo "sigil md5: $(md5sum "$SIGIL" | cut -d' ' -f1)"
cd "$P" || exit 9
for p in "$@"; do
  asl=$(/usr/bin/grep -oE 'ASL_EXIT=[0-9]+' "$p.asl.out")
  out=$("$SIGIL" "$p.asm" --hex 2>&1 | head -2 | tr '\n' ' ')
  printf '%-22s %s  sigil: %s\n' "$p" "$asl" "$out"
done
