#!/usr/bin/env bash
# run_census.sh : SCAFFOLD RUN. Apply the trail-label census hook to a clean
# committed tree, run the frontend and cli suites with it on, then restore the
# file from HEAD and require the tracked tree clean again.
S=/home/volence/sonic_hacks/.scratch/as-macro-diag
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a46ef34b369e42312
F=crates/sigil-span/src/lib.rs
cd "$W" || exit 1
LOG="$S/logs/census-run.log"
{
  echo "HEAD: $(git rev-parse HEAD)  branch: $(git branch --show-current)"
  echo "tracked-changes before scaffold: $(git status --porcelain --untracked-files=no | wc -l)"
} > "$LOG"
if [ "$(git status --porcelain --untracked-files=no | wc -l)" != 0 ]; then
  echo "REFUSED: tree not clean" >> "$LOG"; echo END_MARKER >> "$LOG"; exit 1
fi
python3 "$S/census_scaffold.py" >> "$LOG" 2>&1 || { echo END_MARKER >> "$LOG"; exit 1; }
git diff --stat >> "$LOG"
rm -f "$S/logs/census.txt"
export SIGIL_TRAIL_CENSUS="$S/logs/census.txt"
unset AEON_DIR EMPYREAN_SUITE_ROOT SIGIL_STRICT_GATE
export CARGO_TARGET_DIR="$S/target"
cargo test --release -p sigil-frontend-as --no-fail-fast > "$S/logs/census-fe.log" 2>&1
echo "fe CARGO_EXIT=$?" >> "$LOG"
SIGIL_ALLOW_PARTIAL=1 cargo test --release -p sigil-cli --no-fail-fast > "$S/logs/census-cli.log" 2>&1
echo "cli CARGO_EXIT=$?" >> "$LOG"
cargo test --release -p sigil-span --no-fail-fast > "$S/logs/census-span.log" 2>&1
echo "span CARGO_EXIT=$?" >> "$LOG"
git show "HEAD:$F" > "$F"
echo "restored $F from HEAD; tracked-changes now: $(git status --porcelain --untracked-files=no | wc -l)" >> "$LOG"
echo "census lines: $(wc -l < "$S/logs/census.txt" 2>/dev/null)" >> "$LOG"
echo END_MARKER >> "$LOG"
