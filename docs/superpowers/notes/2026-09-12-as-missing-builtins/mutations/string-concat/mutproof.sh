#!/usr/bin/env bash
# mutproof.sh <label> <repo-relative-file> <test-binary> : apply
# mut/<label>.old -> mut/<label>.new to the file (mutate.py refuses unless the
# old text occurs exactly once, and quotes the mutated lines back FROM DISK),
# show `git diff --stat`, run ONE test binary of sigil-frontend-as, then restore
# the file from the COMMITTED baseline (git show HEAD:<file>) and report the
# tracked-change count, which must be 0. Refuses a dirty tracked tree up front.
S=/home/volence/sonic_hacks/.scratch/as-string-concat
W=/home/volence/sonic_hacks/sigil/.worktrees/as-string-concat
L=$1; F=$2; T=$3
LOG="$S/mut/red-$L.log"
{
  echo "== mutation $L on $F, test binary $T"
  echo "== HEAD $(git -C "$W" rev-parse HEAD) branch $(git -C "$W" branch --show-current)"
  if [ -n "$(git -C "$W" status --porcelain --untracked-files=no)" ]; then
    echo "TREE NOT CLEAN BEFORE MUTATION"; git -C "$W" status --porcelain --untracked-files=no; exit 7
  fi
  python3 "$S/mut/mutate.py" "$W/$F" "$S/mut/$L.old" "$S/mut/$L.new" || { echo "MUTATION FAILED TO APPLY"; exit 5; }
  echo "git diff --stat:"; git -C "$W" diff --stat
  (cd "$W" && CARGO_TARGET_DIR="$W/target" cargo test -p sigil-frontend-as --test "$T" 2>&1) \
    | grep -E '^test |test result|^error(\[|:)|panicked at|assertion|^  left:|^ right:|differs from|refused|built an image|asl refuses|  (asl|sigil) '
  echo "TEST_PIPE_DONE"
  git -C "$W" show "HEAD:$F" > "$W/$F"
  echo "restored $F from HEAD; tracked changes now: $(git -C "$W" status --porcelain --untracked-files=no | wc -l)"
} > "$LOG" 2>&1
cat "$LOG"
