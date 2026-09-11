#!/usr/bin/env bash
# mutproof.sh <label> <repo-relative-file> <test-name> : apply mut/<label>.old ->
# mut/<label>.new to the file (mutate.py refuses unless the old text occurs
# exactly once and quotes the mutated lines back from disk), run ONE test
# binary of sigil-frontend-as, then restore the file from the COMMITTED
# baseline (git show HEAD:<file>) and require a clean tracked tree.
S=/home/volence/sonic_hacks/.scratch/s2-as-small-features
WT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a35527f94a0279a97
L=$1; F=$2; T=$3
LOG="$S/mut/red-$L.log"
cd "$WT" || exit 9
{
  echo "== mutation $L on $F, test $T, HEAD $(git -C "$WT" rev-parse HEAD)"
  if [ -n "$(git -C "$WT" status --porcelain --untracked-files=no)" ]; then
    echo "TREE NOT CLEAN BEFORE MUTATION"; git -C "$WT" status --porcelain --untracked-files=no; exit 7
  fi
  python3 "$S/mutate.py" "$WT/$F" "$S/mut/$L.old" "$S/mut/$L.new" || { echo "MUTATION FAILED TO APPLY"; exit 5; }
  CARGO_TARGET_DIR="$S/target" cargo test --release -p sigil-frontend-as --test "$T" 2>&1 | /usr/bin/grep -E '^test |test result|error\[|panicked'
  echo "TEST_PIPE_DONE"
  git -C "$WT" show "HEAD:$F" > "$WT/$F"
  echo "restored $F from HEAD; tracked changes now: $(git -C "$WT" status --porcelain --untracked-files=no | wc -l)"
} > "$LOG" 2>&1
cat "$LOG"
