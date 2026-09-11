#!/usr/bin/env bash
# run_suite.sh <label> : the scoped suite, stamped with the tree it ran from,
# detached-safe (writes an end marker the caller polls for).
S=/home/volence/sonic_hacks/.scratch/s2-as-small-features
WT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a35527f94a0279a97
LOG="$S/suite-$1.log"
cd "$WT" || exit 9
{
  echo "pwd=$(pwd)"
  echo "head=$(git -C "$WT" rev-parse HEAD) branch=$(git -C "$WT" branch --show-current)"
  echo "tracked-changes=$(git -C "$WT" status --porcelain --untracked-files=no | wc -l)"
  git -C "$WT" status --porcelain --untracked-files=no
} > "$LOG"
SIGIL_ALLOW_PARTIAL=1 CARGO_TARGET_DIR="$S/target" cargo test --release -p sigil-frontend-as -p sigil-cli --no-fail-fast -- --nocapture >> "$LOG" 2>&1
echo "SUITE_END rc=$?" >> "$LOG"
