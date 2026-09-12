#!/usr/bin/env bash
# Run each named probe through the pinned reference asl; print diagnostics only.
cd /home/volence/sonic_hacks/.scratch/as-macro-diag/probes || exit 1
. /home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a46ef34b369e42312/docs/superpowers/notes/asl-reference/asl_ref.sh || exit $?
echo "asl md5: $(md5sum "$ASL" | cut -d' ' -f1)  path: $ASL"
for f in "$@"; do
    echo "=== $f.asm"
    asl_run -xx -n -q -A -L -U -i . "$f.asm" 2>&1 | grep -vE '^  |^REFUSED'
done
