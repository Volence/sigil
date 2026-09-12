#!/usr/bin/env bash
# mutproof.sh <label> <repo-relative-file> <test-binary> : apply mut/<label>.old
# -> mut/<label>.new to the file (mutate.py refuses unless the old text occurs
# exactly once, and quotes the mutated lines back from disk), show git diff
# --stat, run ONE test binary of sigil-frontend-as, then restore the file from
# the COMMITTED baseline (git show HEAD:<file>) and require a clean tracked tree.
S=/home/volence/sonic_hacks/.scratch/as-missing-builtins
WT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a09c77cfcd76b4fb6
L=$1; F=$2; T=$3
LOG="$S/mut/red-$L.log"
cd "$WT" || exit 9
{
  echo "== mutation $L on $F, test binary $T, HEAD $(git -C "$WT" rev-parse HEAD) branch $(git -C "$WT" branch --show-current)"
  if [ -n "$(git -C "$WT" status --porcelain --untracked-files=no)" ]; then
    echo "TREE NOT CLEAN BEFORE MUTATION"; git -C "$WT" status --porcelain --untracked-files=no; exit 7
  fi
  python3 "$S/mutate.py" "$WT/$F" "$S/mut/$L.old" "$S/mut/$L.new" || { echo "MUTATION FAILED TO APPLY"; exit 5; }
  echo "git diff --stat:"; git -C "$WT" diff --stat
  CARGO_TARGET_DIR="$S/target" cargo test --release -p sigil-frontend-as --test "$T" 2>&1 \
    | /usr/bin/grep -E '^test |test result|error(\[|:)|panicked at|differs from|refuses it|refused:|  (asl|sigil) '
  echo "TEST_PIPE_DONE"
  git -C "$WT" show "HEAD:$F" > "$WT/$F"
  echo "restored $F from HEAD; tracked changes now: $(git -C "$WT" status --porcelain --untracked-files=no | wc -l)"
} > "$LOG" 2>&1
cat "$LOG"
