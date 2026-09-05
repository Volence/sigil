#!/bin/sh
# probe runner: run.sh <basename>  — assembles <base>.asm with the reference
# asl and prints exit code + listing.
#
# THE ASSEMBLER IS SELECTED BY DIGEST, NOT BY BANNER. Four asl binaries in this
# workspace print `Macro Assembler 1.42 Beta [Bld 212]` verbatim and one of them
# answers differently on every run for any operand it declined to give a value,
# so a runner that checked the version string would have verified nothing. See
# `../asl-reference/`.
#
# THE PROBES ARE THIS DIRECTORY'S OWN. The predecessor named a `probes/`
# subdirectory of a different worktree — an untracked copy of the committed
# `.asm` files, which can drift from them with nothing to say so.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
cd "$HERE" || exit 1
rm -f "$1.lst" "$1.p" "$1.bin"
USEANSI=n "$ASL" -L -U -q "$1.asm"
echo "ASL_EXIT=$?"
echo "===== LISTING $1.lst ====="
cat "$1.lst" 2>/dev/null
