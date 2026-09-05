#!/bin/bash
# Red-first proof for the AS-MOMPASS parcel.
#
# Three mutations, each applied to the COMMITTED source on disk, each shown
# applied by a content grep AND by `git diff HEAD --stat` (plain `git diff`
# reports nothing after `git checkout <rev> -- <path>`, because that STAGES).
# Each is restored from the committed baseline 876ae0c8, never with a bare
# `git checkout --` on a dirty tree.
#
# A mutation that fails to apply runs the original file and prints ok, which is
# indistinguishable from a clean restore. So every round prints the mutated line
# back before running, and the round is void if that grep finds nothing.
set -u
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f
SRC=crates/sigil-frontend-as/src/eval.rs
BASE=876ae0c8412dd6339611d7fbe185aa75c95b404e
export CARGO_TARGET_DIR=$W/.target-land
cd "$W" || exit 9

echo "baseline: $(git rev-parse --short HEAD)   tree clean: $(git status --porcelain -- $SRC | wc -l) dirty paths in $SRC"
echo

round () {
  local name="$1" py="$2" must_fail="$3"
  echo "===================================================================="
  echo "MUTATION: $name"
  python3 -c "$py" || { echo "  MUTATION FAILED TO APPLY: round VOID"; return 1; }
  echo "  applied, by content grep:"
  grep -n 'MOMPASS" =>' $SRC | sed 's/^/    /'
  grep -nE '^const (FIRST|LATER)_PASS' $SRC | sed 's/^/    /'
  echo "  applied, by git diff HEAD --stat:"
  git diff HEAD --stat -- $SRC | sed 's/^/    /'
  echo "  this run MUST FAIL, and specifically at: $must_fail"
  cargo test -p sigil-frontend-as --test as_mompass_builtin 2>&1 \
    | grep -E '^test |^test result|FAILED|panicked at|left:|right:' | sed 's/^/    /'
  echo "  restoring from committed baseline $BASE"
  git checkout $BASE -- $SRC
  git reset -q HEAD -- $SRC
  echo "  restored, dirty paths in \$SRC now: $(git status --porcelain -- $SRC | wc -l)"
  echo
}

# 1. The builtin is not registered at all: back to the pre-parcel world, where
#    `if MOMPASS=1` refuses as an unresolved condition.
round "MOMPASS is not a builtin" '
import sys
p="crates/sigil-frontend-as/src/eval.rs"; s=open(p).read()
old="            \"MOMPASS\" => Some(self.mompass),\n"
assert s.count(old)==1, "anchor not unique"
open(p,"w").write(s.replace(old,""))
' "every MOMPASS test, as a refusal instead of bytes"

# 2. The saturation is inverted: the FIRST pass reports 2 and later ones 1.
#    This is the mutation a wrong sense of the flag would produce, and it is
#    chosen because a small integer swapped for another small integer is exactly
#    the confoundable shape: it must move the =1 and >1 rows, not just one.
round "the two pass numbers are swapped" '
p="crates/sigil-frontend-as/src/eval.rs"; s=open(p).read()
old="const FIRST_PASS: i64 = 1;\nconst LATER_PASS: i64 = 2;"
new="const FIRST_PASS: i64 = 2;\nconst LATER_PASS: i64 = 1;"
assert s.count(old)==1, "anchor not unique"
open(p,"w").write(s.replace(old,new))
' "mompass_eq_one_is_false, mompass_gt_one_is_true, mompass_reads_as_two"

# 3. The saturation is removed in favour of a running count. This is the design
#    the parcel REJECTED, so it has to be the one that reds: it must break the
#    corpus`s `=2` shape while leaving `=1` and `>1` alone, because a running
#    count still exceeds 1 on the converged pass.
round "a running count instead of the saturation" '
p="crates/sigil-frontend-as/src/eval.rs"; s=open(p).read()
old="            if pass == 0 { FIRST_PASS } else { LATER_PASS },"
new="            pass as i64 + 1,"
assert s.count(old)==1, "anchor not unique"
open(p,"w").write(s.replace(old,new))
' "mompass_eq_two_guarding_an_emission (the =2 shape a running count loses)"

echo "===================================================================="
echo "final state: $(git rev-parse --short HEAD)  dirty paths in \$SRC: $(git status --porcelain -- $SRC | wc -l)"
echo "GREEN after restore:"
cargo test -p sigil-frontend-as --test as_mompass_builtin 2>&1 | grep -E '^test result' | sed 's/^/    /'
echo REDFIRST-END
