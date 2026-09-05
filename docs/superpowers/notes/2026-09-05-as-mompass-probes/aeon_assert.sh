#!/bin/bash
# Bar 5: the aeon effect is asserted, not assumed.
#
# aeon is on this crate's shipping build path, so a change to the AS front end
# has to be shown not to move it. `MOMPASS` appears in no aeon assembly source
# (only in four markdown documents), and aeon's AS-routed surface is three
# roots. This runs the before and after binaries over each root and compares.
#
# A standalone run defines fewer symbols than the real build, so it can only
# OVER-report: identical before/after output is therefore meaningful, and a
# clean result cannot be an artifact of the run being too weak.
#
# This does not build aeon and sets no AEON_DIR. The roots are read only, and
# aeon's worktree cleanliness is printed before and after to prove it.
set -u
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f
BEFORE=$W/.runlogs/sigil-before
AFTER=$W/.target-land/release/sigil
A=/home/volence/sonic_hacks/aeon
OUT=$W/.runlogs/aeon
mkdir -p "$OUT"

echo "aeon HEAD: $(cd $A && git rev-parse HEAD)"
echo "aeon dirty paths BEFORE: $(cd $A && git status --porcelain | wc -l)"
echo "MOMPASS in aeon .asm/.inc/.emp (tracked, excluding agent worktrees):"
(cd $A && git grep -l MOMPASS -- '*.asm' '*.inc' '*.emp' | sed 's/^/    /') || echo "    (none)"
echo

for root in engine/debug/debugger.asm games/demo/game_root.asm games/sonic4/game_root.asm; do
  stem=$(echo "$root" | tr '/' '_')
  cd "$A/$(dirname $root)" || exit 9
  f=$(basename "$root")
  "$BEFORE" "$f" > "$OUT/$stem.before.out" 2> "$OUT/$stem.before.err"; rb=$?
  "$AFTER"  "$f" > "$OUT/$stem.after.out"  2> "$OUT/$stem.after.err";  ra=$?
  db=$(wc -l < "$OUT/$stem.before.err"); da=$(wc -l < "$OUT/$stem.after.err")
  mb=$(grep -c MOMPASS "$OUT/$stem.before.err"); ma=$(grep -c MOMPASS "$OUT/$stem.after.err")
  echo "$root"
  echo "   before: exit=$rb  diagnostics=$db  MOMPASS firings=$mb"
  echo "   after : exit=$ra  diagnostics=$da  MOMPASS firings=$ma"
  if cmp -s "$OUT/$stem.before.err" "$OUT/$stem.after.err" \
     && cmp -s "$OUT/$stem.before.out" "$OUT/$stem.after.out"; then
    echo "   stdout AND stderr byte-identical before vs after: YES"
  else
    echo "   stdout AND stderr byte-identical before vs after: NO  <== READ THIS"
    diff "$OUT/$stem.before.err" "$OUT/$stem.after.err" | head -20 | sed 's/^/     /'
  fi
done

echo
echo "aeon dirty paths AFTER: $(cd $A && git status --porcelain | wc -l)"
echo AEON-ASSERT-END
