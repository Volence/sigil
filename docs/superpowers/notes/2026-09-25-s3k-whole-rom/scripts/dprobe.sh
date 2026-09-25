#!/usr/bin/env bash
# dprobe.sh: what asl's -D does, for sizing a sigil -D (asl_run only; bytes quoted from
# exit-0 runs only). Each case: probe file, then the -D arguments.
set -u
S=/home/volence/sonic_hacks/.scratch/s3k-whole-rom
NOTES=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ae5ad3556fc25d7da/docs/superpowers/notes
. "$NOTES/asl-reference/asl_ref.sh" || exit $?
cd "$S/probes" || exit 1
run() {
  local label="$1" P="$2"; shift 2
  local B="${P%.asm}"
  rm -f "$B.p" "$B.bin" "$B.lst"
  echo "### $label: asl $* $P   ($(sed -n '2,$p' "$P" | tr '\n\t' '| '))"
  if asl_run -xx -n -q -A -L -U -i . "$@" "$P" 2>"$B.err"; then
    "$ASLDIR/p2bin" -p=0 "$B.p" "$B.bin" >/dev/null
    echo "  exit 0: $(xxd -p "$B.bin" | tr -d '\n')"
  else
    echo "  refused: $(grep -hoE 'ASL_EXIT=[0-9]+' "$B.err")"
    grep -hE 'error|warning' "$B.lst" 2>/dev/null | grep -vE '^ *[0-9]+/' | head -3 | sed 's/^/    /'
    grep -hE 'error #|warning #' "$B.err" | head -3 | sed 's/^/    /'
  fi
}
run "value given"            d1.asm -D FOO=5
run "no value"               d1.asm -D FOO
run "comma list"             d2.asm -D FOO=1,BAR=2
run "hex value"              d1.asm -D 'FOO=$10'
run "in-file = after -D"     d3.asm -D FOO=5
run "case (-U): -D FOO, use foo" d4.asm -D FOO=5
run "if on -D symbol, 0"     d5.asm -D FOO=0
run "if on -D symbol, 1"     d5.asm -D FOO=1
run "in-file set after -D"   d6.asm -D FOO=5
run "in-file equ after -D"   d7.asm -D FOO=5
run "-D then use, set, use"  d8.asm -D FOO=5
echo DPROBE_END
