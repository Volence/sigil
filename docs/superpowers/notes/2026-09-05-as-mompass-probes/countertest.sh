#!/bin/bash
# The one measurement the design rests on, taken rather than reasoned.
#
# `m_flag2.asm` mirrors s2.asm(91270): an `if MOMPASS=2` whose body emits NO
# bytes (it sets a `:=` that a later `dc.b` reads), in a file whose iteration
# count is set by a forward `:=` chain rather than by MOMPASS itself. So the
# guard cannot perturb the layout, and sigil's iteration count exceeds asl's
# pass count for a reason unrelated to MOMPASS: exactly the corpus situation.
#
# asl: exit 0, 2 passes, 0 errors, bytes AA 11 03 EE.
set -u
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f
SRC=crates/sigil-frontend-as/src/eval.rs
BASE=876ae0c8412dd6339611d7fbe185aa75c95b404e
export CARGO_TARGET_DIR=$W/.target-land
cd "$W" || exit 9
P=$W/.mompassprobe/m_flag2.asm

echo "== saturating at 2 (what this parcel landed) =="
cargo build --release -p sigil-cli 2>&1 | grep -E '^error'
echo "  bytes: $($W/.target-land/release/sigil $P --hex 2>&1)  exit=$?"

echo
echo "== the same source with a RUNNING COUNT instead =="
python3 -c '
p="crates/sigil-frontend-as/src/eval.rs"; s=open(p).read()
old="            if pass == 0 { FIRST_PASS } else { LATER_PASS },"
new="            pass as i64 + 1,"
assert s.count(old)==1, "anchor not unique"
open(p,"w").write(s.replace(old,new))
' || { echo "  MUTATION FAILED TO APPLY: VOID"; exit 1; }
echo "  applied, by content grep:"
grep -n 'pass as i64 + 1' $SRC | sed 's/^/    /'
echo "  applied, by git diff HEAD --stat:"
git diff HEAD --stat -- $SRC | sed 's/^/    /'
cargo build --release -p sigil-cli 2>&1 | grep -E '^error'
echo "  bytes: $($W/.target-land/release/sigil $P --hex 2>&1)  exit=$?"

echo
echo "  restoring from committed baseline $BASE"
git checkout $BASE -- $SRC
git reset -q HEAD -- $SRC
echo "  dirty paths in \$SRC: $(git status --porcelain -- $SRC | wc -l)"
cargo build --release -p sigil-cli 2>&1 | grep -E '^error'
echo "  bytes after restore: $($W/.target-land/release/sigil $P --hex 2>&1)"
echo COUNTERTEST-END
