#!/usr/bin/env bash
# asl_plain.sh: the plain root with NO -D, through asl_run, to show asl needs the define
# too (diagnostics only are quoted from a refused run).
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
NOTES=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ae5ad3556fc25d7da/docs/superpowers/notes
. "$NOTES/asl-reference/asl_ref.sh" || exit $?
W=$S/trees/sk-aslplain
rm -rf "$W"; cp -a "$S/trees/sk-gen" "$W"
cd "$W" || exit 1
rm -f sonic3k.p sonic3k.log
asl_run -xx -n -q -A -L -U -E -i . sonic3k.asm; echo "asl rc=$?"
if [ -f sonic3k.log ]; then
  echo "sonic3k.log rows: $(grep -c . sonic3k.log)"
  grep -oE 'error #[0-9]+: [^:]*(: [A-Za-z0-9_]+)?' sonic3k.log | sort | uniq -c | sort -rn
  echo "--- first 12 lines"; head -12 sonic3k.log
fi
[ -f sonic3k.p ] && echo "sonic3k.p WAS written" || echo "no sonic3k.p (no object: asl refused)"
echo ASLPLAIN_END
