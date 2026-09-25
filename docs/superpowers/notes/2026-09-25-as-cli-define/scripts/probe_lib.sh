# arguments that precede the source (the -D spelling under test).
set -u
P=/home/volence/sonic_hacks/.scratch/as-cli-define/probes
NOTES=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a746dc1af1279ab3d/docs/superpowers/notes
. "$NOTES/asl-reference/asl_ref.sh" || exit $?
cd "$P" || exit 1
echo "pwd=$(pwd) asl=$ASL md5=$(md5sum "$ASL" | cut -d' ' -f1)"
run() {
  local label="$1" F="$2"; shift 2
  local B="${F%.asm}"
  rm -f "$B.p" "$B.bin" "$B.lst"
  echo "### $label: asl [$*] $F   ($(sed -n '2,$p' "$F" | tr '\n\t' '| '))"
  if asl_run -xx -n -q -A -L -U -i . "$@" "$F" 2>"$B.err"; then
    "$ASLDIR/p2bin" -p=0 "$B.p" "$B.bin" >/dev/null
    echo "  exit 0: $(xxd -p "$B.bin" | tr -d '\n')  [$(grep -hoE 'ASL_DIAG=[a-z]+' "$B.err")] passes: $(grep -hoE '^ +[0-9]+ passe?s?$' "$B.lst" | tr -d ' ')"
    grep -hE 'warning #' "$B.err" | head -3 | sed 's/^/    /'
  else
    echo "  refused: $(grep -hoE 'ASL_EXIT=[0-9]+' "$B.err")"
    grep -hE 'error #|warning #|rror' "$B.err" | grep -v 'REFUSED\|NO BYTE' | head -4 | sed 's/^/    /'
    grep -hvE '^ *$|ASL_|REFUSED|NO BYTE|lines that|reference is|listing prints|beside asl|Fix the source|^  ' "$B.err" | head -4 | sed 's/^/    other: /'
  fi
}
