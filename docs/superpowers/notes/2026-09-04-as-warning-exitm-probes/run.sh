#!/bin/sh
# probe runner: run.sh <basename>  — assembles probes/<base>.asm with the
# committed asl 1.42 Beta Bld 212 and prints exit code + listing.
set -u
D=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-af765a391dc6a9497/probes
ASL=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64/asl
cd "$D" || exit 1
rm -f "$1.lst" "$1.p" "$1.bin"
AS_MSGPATH=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64 USEANSI=n "$ASL" -L -U -q "$1.asm"
echo "ASL_EXIT=$?"
echo "===== LISTING $1.lst ====="
cat "$1.lst" 2>/dev/null
