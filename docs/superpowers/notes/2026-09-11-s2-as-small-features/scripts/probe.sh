#!/usr/bin/env bash
# probe.sh <sigil-binary> <probe.asm> ... : each probe through the REFERENCE asl
# (asl_run: md5-pinned, exit status reported, pass-loop completeness classified)
# and then through the given sigil binary (--hex). One construct per file:
# any asl error stops the pass loop and voids every value in the same file.
S=/home/volence/sonic_hacks/.scratch/s2-as-small-features
WT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a35527f94a0279a97
SIGIL="$1"; shift
. "$WT/docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?
echo "asl md5: $(md5sum "$ASL" | cut -d' ' -f1)  path: $ASL"
echo "sigil md5: $(md5sum "$SIGIL" | cut -d' ' -f1)  path: $SIGIL"
cd "$S/probes" || exit 9
if [ $# -eq 0 ]; then set -- $(/usr/bin/ls *.asm); fi
if [ "${1#@}" != "$1" ]; then set -- $(cat "${1#@}"); fi
for p in "$@"; do
  b=${p%.asm}
  echo "================ $p"
  rm -f "$b.lst" "$b.p" "$b.log" "$b.h"
  asl_run -xx -n -q -A -L -U -i . "$p" > "$b.asl.out" 2>&1
  echo "asl exit=$? $(/usr/bin/grep -E '^ASL_(EXIT|DIAG)' "$b.asl.out" | tr '\n' ' ')"
  /usr/bin/grep -E 'error|warning' "$b.asl.out" | /usr/bin/grep -v '^REFUSED\|^ASL_\|NO BYTE COLUMN' | head -12
  if [ -f "$b.lst" ]; then
    /usr/bin/grep -E '^ +[0-9]+/ +[0-9A-F]+ : ' "$b.lst" | head -60
  fi
  "$SIGIL" "$p" --hex > "$b.sigil.out" 2>&1
  echo "sigil exit=$?"
  cat "$b.sigil.out" | head -30
done
echo PROBE_END
