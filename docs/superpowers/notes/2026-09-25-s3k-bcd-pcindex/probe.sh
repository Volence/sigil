#!/usr/bin/env bash
# probe.sh <sigil binary> <name>...: assemble probes/<name>.asm with the pinned asl
# (asl_run) and with the given sigil; print both exits, diagnostics, both images as
# hex. asl bytes come from p2bin over asl's .p, and ONLY when asl exited 0 with its
# pass loop complete.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex
W=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-adae49c7bddc9a0a8
. "$W/docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?
P2BIN=$ASLDIR/p2bin
SIGIL="$1"; shift
echo "asl md5: $(md5sum "$ASL" | cut -d' ' -f1)  sigil md5: $(md5sum "$SIGIL" | cut -d' ' -f1)"
cd "$S/probes" || exit 9
NAMES=("$@")
if [ "${NAMES[0]:-}" = ALL ]; then
  NAMES=()
  for f in *.asm; do NAMES+=("${f%.asm}"); done
fi
echo "probes: ${#NAMES[@]}"
for N in "${NAMES[@]}"; do
  rm -f "$N.p" "$N.lst" "$N.asl.bin" "$N.sigil.bin" "$N.h"
  echo "=== $N ($(md5sum "$N.asm" | cut -c1-8))"
  asl_run -xx -n -q -A -L -U -i . "$N.asm" > "$N.asl.out" 2>&1
  arc=$?
  grep -E 'ASL_EXIT|ASL_DIAG|rror|arning' "$N.asl.out" | grep -v '^  ' | head -12
  if [ $arc -eq 0 ] && grep -q 'ASL_DIAG=complete' "$N.asl.out"; then
    "$P2BIN" "$N.p" "$N.asl.bin" -p=0 > /dev/null 2>&1
    echo "asl   bytes: $(xxd -p -c 1024 "$N.asl.bin" | tr -d '\n')"
  else
    echo "asl   bytes: (asl exited $arc; no value quoted)"
  fi
  "$SIGIL" "$N.asm" -o "$N.sigil.bin" > "$N.sigil.out" 2>&1
  src=$?
  echo "SIGIL_EXIT=$src"
  grep -E 'error|warning' "$N.sigil.out" | head -8
  if [ -f "$N.sigil.bin" ] && [ $src -eq 0 ]; then
    echo "sigil bytes: $(xxd -p -c 1024 "$N.sigil.bin" | tr -d '\n')"
  fi
  if [ $arc -eq 0 ] && [ $src -eq 0 ]; then
    if cmp -s "$N.asl.bin" "$N.sigil.bin"; then echo "MATCH"; else echo "DIFFER"; fi
  elif [ $arc -ne 0 ] && [ $src -ne 0 ]; then
    echo "BOTH-REFUSE"
  else
    echo "EXIT-MISMATCH"
  fi
done
echo PROBE_END
