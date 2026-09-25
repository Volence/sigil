#!/usr/bin/env bash
# probe.sh <probe.asm>: asl_run (the blessed invocation) + p2bin -p=0, then sigil; hex of each.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
NOTES=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ae5ad3556fc25d7da/docs/superpowers/notes
. "$NOTES/asl-reference/asl_ref.sh" || exit $?
P="$1"; B="${P%.asm}"
cd "$S/probes" || exit 1
rm -f "$B.p" "$B.bin" "$B.lst" "$B.sigil.bin"
asl_run -xx -n -q -A -L -U -i . "$P"; RC=$?
if [ "$RC" -eq 0 ]; then
  "$ASLDIR/p2bin" -p=0 "$B.p" "$B.bin" >/dev/null
  echo "asl   (exit 0): $(xxd -p "$B.bin" | tr -d '\n')"
else
  echo "asl   exit $RC: no bytes quoted"
fi
"$S/target/release/sigil" "$P" -o "$B.sigil.bin" 2>&1 | grep -E 'error|warning' | head -5
if [ -f "$B.sigil.bin" ]; then echo "sigil        : $(xxd -p "$B.sigil.bin" | tr -d '\n')"; else echo "sigil: no image"; fi
echo PROBE_END
