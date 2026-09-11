#!/usr/bin/env bash
# probe.sh <probe.asm> ... : each probe through the REFERENCE asl (asl_run,
# md5-pinned, exit-status reported, pass-loop completeness classified) and
# through the census sigil binary (--hex). One construct per file, because any
# asl error stops the pass loop and voids every value in the same file.
S=/home/volence/sonic_hacks/.scratch/s2-residual-census
WT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a106286d7f094d71e
SIGIL=$S/target/release/sigil
. "$WT/docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?
cd "$S/probes" || exit 9
for p in "$@"; do
  b=${p%.asm}
  echo "================ $p"
  rm -f "$b.lst" "$b.p" "$b.log"
  asl_run -xx -n -q -A -L -U -i . "$p" > "$b.asl.out" 2>&1
  echo "asl exit=$? $(/usr/bin/grep -E '^ASL_(EXIT|DIAG)' "$b.asl.out" | tr '\n' ' ')"
  /usr/bin/grep -E 'error|warning' "$b.asl.out" | /usr/bin/grep -v '^REFUSED\|^ASL_' | head -8
  if [ -f "$b.lst" ]; then
    /usr/bin/grep -E '^ +[0-9]+/ +[0-9A-F]+ : [0-9A-F=]' "$b.lst" | head -40
  fi
  "$SIGIL" "$p" --hex > "$b.sigil.out" 2>&1
  echo "sigil exit=$?"
  cat "$b.sigil.out"
done
echo PROBE_END
