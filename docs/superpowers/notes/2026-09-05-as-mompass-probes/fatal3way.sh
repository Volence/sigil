#!/bin/bash
# Three-way comparison for the terminal-`fatal` fix.
#
#   MASTER   = sigil master 3c3f625c, MOMPASS unimplemented
#   MOMPASS  = branch tip 9f579155, MOMPASS implemented, fatal still droppable
#   FATALFIX = MOMPASS plus the terminal-`fatal` stop
#
# The question the coordinator asked is whether the fix can be made without a
# static census of every corpus `fatal`. The measurement that replaces the
# census is this one: every root there is, all three binaries, exit codes and
# diagnostic sets compared. A fix that changes nothing anywhere except the shape
# it was written for does not need a census to bound it.
set -u
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a8a6251ffb509859f
MASTER=$W/.runlogs/sigil-before
MOMPASS=$W/.runlogs/sigil-mompass-nofix
FATALFIX=$W/.runlogs/sigil-fatalfix
OUT=$W/.runlogs/fatal3
mkdir -p "$OUT"

echo "MASTER   md5: $(md5sum $MASTER   | cut -d' ' -f1)  (sigil master 3c3f625c)"
echo "MOMPASS  md5: $(md5sum $MOMPASS  | cut -d' ' -f1)  (branch tip 9f579155)"
echo "FATALFIX md5: $(md5sum $FATALFIX | cut -d' ' -f1)  (branch + terminal fatal)"
echo

run () {
  local label="$1" dir="$2" root="$3"
  echo "-------------------------------------------------------------"
  echo "$label   ($dir/$root)"
  cd "$dir" || { echo "  MISSING TREE, row VOID"; return; }
  for w in MASTER MOMPASS FATALFIX; do
    case $w in
      MASTER)   BIN=$MASTER ;;
      MOMPASS)  BIN=$MOMPASS ;;
      FATALFIX) BIN=$FATALFIX ;;
    esac
    "$BIN" "$root" > "$OUT/$label.$w.out" 2> "$OUT/$label.$w.err"; rc=$?
    n=$(wc -l < "$OUT/$label.$w.err")
    printf '   %-9s exit=%d  diagnostics=%d\n' "$w" "$rc" "$n"
  done
  if cmp -s "$OUT/$label.MOMPASS.err" "$OUT/$label.FATALFIX.err" \
     && cmp -s "$OUT/$label.MOMPASS.out" "$OUT/$label.FATALFIX.out"; then
    echo "   MOMPASS vs FATALFIX: stdout AND stderr byte-identical"
  else
    echo "   MOMPASS vs FATALFIX: DIFFER  <== READ THIS"
    diff "$OUT/$label.MOMPASS.err" "$OUT/$label.FATALFIX.err" | head -20 | sed 's/^/     /'
  fi
}

run s2disasm /home/volence/sonic_hacks/s2disasm-mompass-clean s2.asm
run s1disasm /home/volence/sonic_hacks/s1disasm sonic.asm
run skdisasm /home/volence/sonic_hacks/skdisasm sonic3k.asm
run aeon-debugger /home/volence/sonic_hacks/aeon/engine/debug debugger.asm
run aeon-demo /home/volence/sonic_hacks/aeon/games/demo game_root.asm
run aeon-sonic4 /home/volence/sonic_hacks/aeon/games/sonic4 game_root.asm

echo "-------------------------------------------------------------"
echo "aeon dirty paths after: $(cd /home/volence/sonic_hacks/aeon && git status --porcelain | wc -l)"
echo FATAL3WAY-END
