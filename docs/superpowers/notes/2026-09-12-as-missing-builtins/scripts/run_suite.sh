#!/usr/bin/env bash
# run_suite.sh <label> : the scoped suite, stamped with the tree it ran from.
S=/home/volence/sonic_hacks/.scratch/as-missing-builtins
WT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a09c77cfcd76b4fb6
LOG="$S/logs/suite-$1.log"
cd "$WT" || exit 9
{
  echo "pwd=$(pwd)"
  echo "head=$(git -C "$WT" rev-parse HEAD) branch=$(git -C "$WT" branch --show-current)"
  echo "tracked-changes=$(git -C "$WT" status --porcelain --untracked-files=no | wc -l)"
  echo "AEON_DIR=${AEON_DIR:-<unset>}"
} > "$LOG"
SIGIL_ALLOW_PARTIAL=1 CARGO_TARGET_DIR="$S/target" cargo test --release -p sigil-frontend-as -p sigil-cli --no-fail-fast -- --nocapture >> "$LOG" 2>&1
echo "SUITE_END rc=$?" >> "$LOG"
python3 "$S/suite_sum.py" "$LOG"
/usr/bin/grep -c 'as_int_builtins' "$LOG" | sed 's/^/log lines naming as_int_builtins: /'
