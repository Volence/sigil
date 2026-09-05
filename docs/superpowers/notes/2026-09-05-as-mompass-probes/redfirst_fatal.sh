#!/bin/bash
# Red-first proof for the terminal-`fatal` fix.
#
# Same bars as the MOMPASS rounds: every mutation is applied to the COMMITTED
# source on disk, shown applied by a content grep AND by `git diff HEAD --stat`
# (plain `git diff` reports nothing after `git checkout <rev> -- <path>`,
# because that STAGES), states what the run MUST fail at before running it, and
# is restored from the committed baseline. A mutation that fails to apply voids
# its round rather than printing ok.
set -u
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f
SRC=crates/sigil-frontend-as/src/eval.rs
BASE=a77ef3b0
export CARGO_TARGET_DIR=$W/.target-land
cd "$W" || exit 9

echo "baseline: $(git rev-parse --short HEAD)   dirty paths in \$SRC: $(git status --porcelain -- $SRC | wc -l)"
echo

round () {
  local name="$1" py="$2" must_fail="$3"
  echo "===================================================================="
  echo "MUTATION: $name"
  python3 -c "$py" || { echo "  MUTATION FAILED TO APPLY: round VOID"; return 1; }
  echo "  applied, by git diff HEAD --stat:"
  git diff HEAD --stat -- $SRC | sed 's/^/    /'
  echo "  applied, by content grep:"
  git diff HEAD -- $SRC | grep -E '^[-+][^-+]' | head -8 | sed 's/^/    /'
  echo "  this run MUST FAIL at: $must_fail"
  cargo test -p sigil-frontend-as --test as_fatal_survives_its_pass 2>&1 \
    | grep -E '^test |^test result|panicked at|must name|the fatal was lost|carried once' | sed 's/^/    /'
  echo "  restoring from committed baseline $BASE"
  git checkout $BASE -- $SRC
  git reset -q HEAD -- $SRC
  echo "  restored, dirty paths in \$SRC: $(git status --porcelain -- $SRC | wc -l)"
  echo
}

# 1. The carry is removed: back to the state the overseer held the branch on,
#    where the fatal vanishes and the run exits 0.
round "a fatal raised on a non-final pass is not carried" '
p="crates/sigil-frontend-as/src/eval.rs"; s=open(p).read()
old="""        if let Some(f) = terminal_fatal {
            if !carried_fatals.contains(&f) {
                carried_fatals.push(f);
            }
        }
"""
assert s.count(old)==1, "anchor not unique"
open(p,"w").write(s.replace(old,"        let _ = terminal_fatal;\n"))
' "a_fatal_on_a_non_final_pass_is_reported (bytes instead of a refusal)"

# 2. The raise-time label is dropped and the bare span is trusted. This is the
#    mutation that separates a fix from a fix that MISATTRIBUTES: it must red
#    ONLY the include-splice test, because a single-file span renders correctly
#    either way. A round that reds everything would not have shown that.
round "trust the returning pass to render the carried span" '
p="crates/sigil-frontend-as/src/eval.rs"; s=open(p).read()
old="        if sources.label(*span).as_ref() == raised_label.as_ref() {"
assert s.count(old)==1, "anchor not unique"
open(p,"w").write(s.replace(old,"        if true {"))
' "a_carried_fatal_names_the_file_it_was_written_in, and ONLY that one"

# 3. The dedupe is removed. A fatal that fires on every pass would then be
#    reported once per pass, which is the failure mode a naive carry has and
#    which no byte comparison would catch.
round "carry without deduping against what the pass already reported" '
p="crates/sigil-frontend-as/src/eval.rs"; s=open(p).read()
old="""        if diags
            .iter()
            .any(|d| d.primary == *span && d.message == *message)
        {
            continue;
        }
"""
assert s.count(old)==1, "anchor not unique"
open(p,"w").write(s.replace(old,""))
' "a_fatal_on_the_final_pass_is_reported_once (reported twice)"

echo "===================================================================="
echo "final: $(git rev-parse --short HEAD)  dirty paths in \$SRC: $(git status --porcelain -- $SRC | wc -l)"
echo "GREEN after restore:"
cargo test -p sigil-frontend-as --test as_fatal_survives_its_pass 2>&1 | grep -E '^test result' | sed 's/^/    /'
cargo test -p sigil-frontend-as --test as_mompass_builtin 2>&1 | grep -E '^test result' | sed 's/^/    /'
echo REDFIRST-FATAL-END
