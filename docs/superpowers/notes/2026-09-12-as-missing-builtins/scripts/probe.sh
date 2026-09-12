#!/usr/bin/env bash
# probe.sh <sigil-binary> <outtag> [probe.asm ...] : each probe through the
# REFERENCE asl (asl_run: md5-pinned, exit status reported, pass-loop
# completeness classified), then through the given sigil binary (--hex).
# One construct per file. Writes a TSV row per probe to results-<outtag>.tsv:
#   probe  asl_exit  asl_diag  asl_bytes_of_construct  asl_errors  sigil_exit  sigil_hex  sigil_first_error
S=/home/volence/sonic_hacks/.scratch/as-missing-builtins
WT=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a09c77cfcd76b4fb6
SIGIL="$1"; TAG="$2"; shift 2
. "$WT/docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?
echo "asl md5: $(md5sum "$ASL" | cut -d' ' -f1)  path: $ASL"
echo "sigil md5: $(md5sum "$SIGIL" | cut -d' ' -f1)  path: $SIGIL"
cd "$S/probes" || exit 9
if [ $# -eq 0 ]; then set -- $(/usr/bin/ls *.asm); fi
OUT="$S/results-$TAG.tsv"
: > "$OUT"
for p in "$@"; do
  b=${p%.asm}
  rm -f "$b.lst" "$b.p" "$b.log" "$b.h"
  asl_run -xx -n -q -A -L -U -i . "$p" > "$b.asl.out" 2>&1
  rc=$?
  diag=$(/usr/bin/grep -oE '^ASL_DIAG=[A-Za-z]+' "$b.asl.out" | head -1 | cut -d= -f2)
  errs=$(/usr/bin/grep -oE '(error|warning) #[0-9]+: [^|]*' "$b.asl.out" | head -3 | tr '\n' ';' | tr '\t' ' ')
  bytes=""
  if [ -f "$b.lst" ]; then
    # the listing row of the line that carries the construct: the first row
    # whose source text is not the header or preamble
    bytes=$(/usr/bin/grep -E '^ +[0-9]+/ +[0-9A-F]+ : ' "$b.lst" \
      | /usr/bin/grep -vE ':\s+\s*(cpu|padding|org|move\.w #\$1234|dc\.w \$5678|dc\.b \$EE|end|F set|X equ|Later equ|signedToString)\b' \
      | head -1 | sed -E 's/^ +[0-9]+\/ +[0-9A-F]+ : //' | tr '\t' ' ')
  fi
  "$SIGIL" "$p" --hex > "$b.sigil.out" 2>&1
  src=$?
  shex=$(/usr/bin/grep -vE 'error|warning' "$b.sigil.out" | tr -d '\n ' | head -c 120)
  serr=$(/usr/bin/grep -E 'error' "$b.sigil.out" | head -1 | tr '\t' ' ')
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$b" "$rc" "$diag" "$bytes" "$errs" "$src" "$shex" "$serr" >> "$OUT"
done
echo "rows: $(wc -l < "$OUT")"
echo PROBE_END
