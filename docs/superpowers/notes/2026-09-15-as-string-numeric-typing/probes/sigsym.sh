#!/usr/bin/env bash
SIGIL=/home/volence/sonic_hacks/.wt-as-string-typing/.target-land/release/sigil
cd "$(dirname "$0")" || exit 2
d=sig_tmp; mkdir -p "$d"; rm -f "$d"/t.bin
{ echo '	cpu 68000'; echo '	padding off'; echo '	org 0'
  echo 'S1 equ "a"'; echo 'S2 equ "ab"'; echo 'S4 equ "abcd"'
  for l in "$@"; do printf '\t%s\n' "$l"; done; echo '	end'; } > "$d/t.asm"
"$SIGIL" "$d/t.asm" -o "$d/t.bin" 2>&1 | head -2
[ -f "$d/t.bin" ] && echo "BYTES: $(xxd -p "$d/t.bin" | tr -d '\n')"
