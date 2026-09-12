#!/usr/bin/env bash
# Red-first mutation proof on the COMMITTED fix. usage: mutation-proof.sh <fix-sha>
# 1. apply the subject mutation (far_scratch_slot accepts every slot = the pre-fix cursor)
# 2. show it applied: git diff --stat naming native.rs, and the mutated line read from disk
# 3. run the two tests; keep the full log
# 4. restore native.rs from the committed fix SHA (not the index) and show the tree clean
set -u
S=/home/volence/sonic_hacks/.scratch/link-zero-byte-move
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a54dcc6c25c9ca708
F=crates/sigil-harness/src/native.rs
sha=$1
log=$S/mutation-proof.log
cd "$W" || exit 9
{
  echo "START $(date +%T) HEAD=$(git rev-parse --short HEAD) fix=$sha branch=$(git branch --show-current)"
  echo "pre-mutation status: $(git status --short | wc -l) dirty paths"
  python3 "$S/mutation-no-skip.py" "$W/$F"
  echo "--- git diff --stat (the mutation, applied):"
  git diff --stat
  echo "--- mutated line on disk:"
  grep -n "if true || one_wrap" "$F"
  echo "--- test run against the mutated subject:"
} > "$log" 2>&1
CARGO_TARGET_DIR=$S/target cargo test --release -p sigil-harness --lib -- \
  derived_layout_tests::a_reference_into_the_far_scratch_measures_abs_l_at_every_slot_ordinal \
  derived_layout_tests::zero_byte_sections_ahead_of_a_never_pinned_target_move_nothing >> "$log" 2>&1
echo "cargo test exit: $?" >> "$log"
{
  echo "--- restore from the committed fix $sha:"
  git restore --source="$sha" -- "$F"
  echo "post-restore status: $(git status --short | wc -l) dirty paths"
  echo "diff against $sha for $F: $(git diff "$sha" -- "$F" | wc -l) lines"
  echo "--- the same two tests on the restored subject:"
} >> "$log" 2>&1
CARGO_TARGET_DIR=$S/target cargo test --release -p sigil-harness --lib -- \
  derived_layout_tests::a_reference_into_the_far_scratch_measures_abs_l_at_every_slot_ordinal \
  derived_layout_tests::zero_byte_sections_ahead_of_a_never_pinned_target_move_nothing >> "$log" 2>&1
echo "cargo test exit (restored): $?" >> "$log"
echo "END-MARKER-MUTATION $(date +%T)" >> "$log"
