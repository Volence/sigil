#!/usr/bin/env bash
# collect.sh: copy scripts, logs and run records into the note's evidence directory.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
E=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ae5ad3556fc25d7da/docs/superpowers/notes/2026-09-25-s3k-whole-rom
mkdir -p "$E/logs" "$E/runs" "$E/scripts"
cp "$S"/scripts/*.sh "$S"/scripts/*.py "$E/scripts/"
for f in "$S"/logs/*.log "$S"/logs/*.txt; do
  [ -f "$f" ] || continue
  case "$f" in *.lst) continue ;; esac
  cp "$f" "$E/logs/"
done
for r in "$S"/runs/*/; do
  n=$(basename "$r")
  mkdir -p "$E/runs/$n"
  cp "$r/provenance" "$r/exit" "$r/rows" "$E/runs/$n/"
done
echo COLLECT_END
