#!/usr/bin/env bash
# probe.sh <probe.asm>: asl_run (blessed invocation) + p2bin -p=0, then sigil base and
# (if built) sigil tip; hex of each. Bytes are quoted only from an exit-0, pass-complete run.
set -u
S=/home/volence/sonic_hacks/.scratch/as-squote
NOTES=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a216a42e1f5962449/docs/superpowers/notes
. "$NOTES/asl-reference/asl_ref.sh" || exit $?
P="$1"; B="${P%.asm}"
cd "$S/probes" || exit 1
rm -f "$B.p" "$B.bin" "$B.lst" "$B".sigil*
echo "== $P"
asl_run -xx -n -q -A -L -U -i . "$P" > "$B.asl.out" 2>&1; RC=$?
if [ "$RC" -eq 0 ] && grep -q "ASL_DIAG=complete" "$B.asl.out"; then
  "$ASLDIR/p2bin" -p=0 "$B.p" "$B.bin" >/dev/null
  echo "asl   (exit 0): $(xxd -p "$B.bin" | tr -d '\n')"
else
  echo "asl   exit $RC: no bytes quoted"; grep -E "rror|arning" "$B.asl.out" | head -6
fi
grep -E "arning" "$B.asl.out" | head -3 | sed 's/^/  asl: /'
run_sigil() {
  local tag="$1" bin="$2"
  [ -x "$bin" ] || return 0
  "$bin" "$P" -o "$B.sigil-$tag.bin" > "$B.sigil-$tag.out" 2>&1; local SRC=$?
  grep -E '(error|warning)' "$B.sigil-$tag.out" | head -4 | sed "s/^/  $tag: /"
  if [ "$SRC" -eq 0 ] && [ -f "$B.sigil-$tag.bin" ]; then
    echo "sigil-$tag (exit 0): $(xxd -p "$B.sigil-$tag.bin" | tr -d '\n')"
  else
    echo "sigil-$tag exit $SRC: no image"
  fi
}
run_sigil base "$S/target-base/release/sigil"
run_sigil tip "$S/target/release/sigil"
echo PROBE_END
