#!/usr/bin/env bash
# sig.sh "<line>" ... : assemble exactly these lines with THIS worktree's sigil
# and hexdump the result. Prints the diagnostics and the exit status.
SIGIL=/home/volence/sonic_hacks/.wt-as-string-typing/.target-land/release/sigil
cd "$(dirname "$0")" || exit 2
d=sig_tmp; mkdir -p "$d"; rm -f "$d"/t.bin
{ echo '	cpu 68000'; echo '	padding off'; echo '	org 0'
  for l in "$@"; do printf '\t%s\n' "$l"; done; echo '	end'; } > "$d/t.asm"
"$SIGIL" "$d/t.asm" -o "$d/t.bin" 2>&1
echo "SIGIL_EXIT=$?"
[ -f "$d/t.bin" ] && echo "BYTES: $(xxd -p "$d/t.bin" | tr -d '\n')"
