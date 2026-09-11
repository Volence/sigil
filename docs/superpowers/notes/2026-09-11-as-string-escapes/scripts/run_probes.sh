#!/usr/bin/env bash
# run_probes.sh [sigil-binary] : every probes/p_*.asm through the pinned asl
# (asl_run: md5 check, exit status, pass-loop completeness), then through the
# given sigil binary with --hex. One construct per file.
S=/home/volence/sonic_hacks/.scratch/as-string-escapes
WT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a3c01b40c82d95852
SIGIL=${1:-$S/bin/sigil-before}
. "$WT/docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?
echo "ASL md5: $(md5sum "$ASL" | cut -d' ' -f1)  path: $ASL"
echo "SIGIL md5: $(md5sum "$SIGIL" | cut -d' ' -f1)  path: $SIGIL"
cd "$S/probes" || exit 9
for p in p_*.asm; do
  b=${p%.asm}
  echo "================ $p"
  sed -n '4,$p' "$p" | /usr/bin/grep -v -E '^\s*(end|dc\.b \$EE|db 0EEh)$' | sed 's/^/  src| /'
  rm -f "$b.lst" "$b.p" "$b.log"
  asl_run -xx -n -q -A -L -U -i . "$p" > "$b.asl.out" 2>&1
  rc=$?
  echo "asl exit=$rc $(/usr/bin/grep -E '^ASL_(EXIT|DIAG)' "$b.asl.out" | tr '\n' ' ')"
  /usr/bin/grep -E 'error|warning|> > >' "$b.asl.out" | /usr/bin/grep -v -E '^(REFUSED|ASL_|  )' | head -6 | sed 's/^/  asl-diag| /'
  /usr/bin/grep -v -E '^(ASL_|REFUSED|  )' "$b.asl.out" | /usr/bin/grep -v -E 'error|warning' | /usr/bin/grep -v '^$' | head -4 | sed 's/^/  asl-out| /'
  if [ -f "$b.lst" ]; then
    # every listing line from the construct up to the sentinel, continuation lines included
    awk '/^ +[0-9]+\/ +[0-9A-F]+ :/{on=1} on{print}' "$b.lst" | /usr/bin/grep -v -E '^\s*$' \
      | awk '/ : EE  /{exit} /Symbol|^ *[0-9]+ (lines|passes|pass)/{exit} {print}' | sed 's/^/  lst| /' | head -30
  fi
  "$SIGIL" "$p" --hex > "$b.sigil.out" 2>&1
  echo "sigil exit=$?"
  sed 's/^/  sigil| /' "$b.sigil.out" | /usr/bin/grep -v 'this error list may be incomplete' | head -6
done
echo PROBE_END
