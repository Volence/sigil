#!/usr/bin/env bash
# one.sh "<line>" [more lines...] : assemble exactly these lines, alone, and
# report the exit status and the body. A failing run's bytes are NOT quotable;
# its refusal is.
. /home/volence/sonic_hacks/sigil/docs/superpowers/notes/asl-reference/asl_ref.sh || exit $?
cd "$(dirname "$0")" || exit 2
d=one_tmp; mkdir -p "$d"; rm -f "$d"/t.lst "$d"/t.p
{ echo '	cpu 68000'; echo '	padding off'; echo '	org 0'
  for l in "$@"; do printf '\t%s\n' "$l"; done; echo '	end'; } > "$d/t.asm"
( cd "$d" && asl_run -xx -n -q -A -L -U -i . t.asm ) 2>&1 | grep -E 'ASL_EXIT|ASL_DIAG|error #|warning #'
sed -n '/Source File/,/Symbol Table/p' "$d/t.lst" | sed '/Symbol Table/,$d' | sed -n '4,$p' | grep -v '^$' | grep -v 'Source File'
