#!/usr/bin/env bash
# mutrun.sh <id>: apply mutation, show it on disk, run as_codepage + state unit
# tests, record failures, restore the file from the committed HEAD.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-codepage
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a08cead7e136bc593
ID="$1"
LOG=$S/logs/mut-$ID.log
cd "$W" || exit 9
{
  echo "pwd: $(pwd)  HEAD: $(git rev-parse HEAD)  branch: $(git branch --show-current)"
  python3 $S/mutate.py "$ID"
  git diff --stat
  git diff -U0 | grep '^[-+][^-+]'
  CARGO_TARGET_DIR=$S/target cargo test --release -p sigil-frontend-as --test as_codepage --no-fail-fast 2>&1 | grep -E '^test |test result|refusal does not|left:|right:|diagnostics:|expected a refusal|panicked'
  CARGO_TARGET_DIR=$S/target cargo test --release -p sigil-frontend-as --lib state:: 2>&1 | grep -E 'FAILED|test result|panicked'
} > "$LOG" 2>&1
for f in crates/sigil-frontend-as/src/state.rs crates/sigil-frontend-as/src/eval.rs; do
  git show "HEAD:$f" > "$f"
done
echo "restored; diff stat after: [$(git diff --stat)]" >> "$LOG"
grep -E 'applied|FAILED|test result|restored|changed' "$LOG"
echo MUT_END_$ID
