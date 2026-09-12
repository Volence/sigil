#!/usr/bin/env bash
# run_m8b.sh : m8 again (dedup keyed on the run id), against the PRE-EXISTING
# count test in as_unresolved_if_condition.rs, which the census found computing a
# trail label and which asserts one verdict for four expansions.
S=/home/volence/sonic_hacks/.scratch/as-macro-diag
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a46ef34b369e42312
cd "$W" || exit 1
export CARGO_TARGET_DIR="$S/target"
LOG="$S/logs/mutations/red-m8b.log"
EVAL=crates/sigil-frontend-as/src/eval.rs
{
  echo "== m8b   HEAD $(git rev-parse HEAD)   branch $(git branch --show-current)"
  echo "tracked-changes before: $(git status --porcelain --untracked-files=no | wc -l)"
} > "$LOG"
[ "$(git status --porcelain --untracked-files=no | wc -l)" = 0 ] || { echo "REFUSED dirty" >> "$LOG"; exit 1; }
python3 "$S/mutate.py" "$EVAL" "        let p = self.sources.physical(span);
        (p.source.0, p.start, p.end)" "        let p = span;
        (p.source.0, p.start, p.end)" >> "$LOG" 2>&1 || exit 1
timeout 900 cargo test --release -p sigil-frontend-as --test as_unresolved_if_condition --no-fail-fast >> "$LOG" 2>&1
echo "exit=$?" >> "$LOG"
git show "HEAD:$EVAL" > "$EVAL"
echo "restored $EVAL from HEAD; tracked-changes after: $(git status --porcelain --untracked-files=no | wc -l)" >> "$LOG"
red=$(grep -E '^test .* FAILED$' "$LOG" | sed -E 's/^test (.*) \.\.\. FAILED$/\1/' | sort -u | tr '\n' ' ')
echo "m8b :: $(awk '/^test result:/{p+=$4; f+=$6} END{print "passed " p ", failed " f}' "$LOG") :: RED: ${red:-<none>}" | tee -a "$LOG"
