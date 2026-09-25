#!/usr/bin/env bash
# mutrun.sh <id...>: for each mutation: apply, show it on disk (diff stat), run the
# parcel's tests, restore from the committed HEAD, show an empty diff stat.
S=/home/volence/sonic_hacks/.scratch/s3k-dollar-labels
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a620ff60a900c195e
cd "$W" || exit 9
echo "pwd=$(pwd) HEAD=$(git rev-parse HEAD) branch=$(git branch --show-current)"
for m in "$@"; do
  echo "=================== $m"
  python3 "$S/mutate.py" "$m" || { echo "APPLY FAILED $m"; continue; }
  echo "on disk: $(git diff --stat | tail -1)"
  CARGO_TARGET_DIR=$S/target cargo test --release -p sigil-frontend-as --test as_dollar_labels --no-fail-fast 2>&1 \
    | grep -E '^test .*FAILED|test result|refusal does not name|differs from|got diagnostics|expected a refusal|bytes differ' | cut -c1-260
  for f in crates/sigil-frontend-as/src/eval.rs crates/sigil-frontend-as/src/lexer.rs; do
    git show "HEAD:$f" > "$f"
  done
  echo "restored: diff stat [$(git diff --stat | tail -1)]"
done
echo MUTRUN_END
