#!/usr/bin/env bash
# sig.sh "<line>" ... : assemble exactly these lines with THIS worktree's sigil
# and hexdump the result. Prints the diagnostics and the exit status.
# The assembler under test. The default is the repo's own release build; a
# worktree run overrides it. This was an absolute path into the worktree the
# parcel was written in, which stops existing the moment that worktree is
# reaped, and these scripts are committed to be re-runnable.
SIGIL="${SIGIL:-$(cd "$(dirname "$0")/../../../../.." && pwd)/target/release/sigil}"
cd "$(dirname "$0")" || exit 2
d=sig_tmp; mkdir -p "$d"; rm -f "$d"/t.bin
{ echo '	cpu 68000'; echo '	padding off'; echo '	org 0'
  for l in "$@"; do printf '\t%s\n' "$l"; done; echo '	end'; } > "$d/t.asm"
"$SIGIL" "$d/t.asm" -o "$d/t.bin" 2>&1
echo "SIGIL_EXIT=$?"
[ -f "$d/t.bin" ] && echo "BYTES: $(xxd -p "$d/t.bin" | tr -d '\n')"
