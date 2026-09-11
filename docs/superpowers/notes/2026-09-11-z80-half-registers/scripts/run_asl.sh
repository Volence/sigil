#!/usr/bin/env bash
# Run the pinned asl through asl_run on each named probe, inside the probe dir.
# usage: run_asl.sh <probe-dir> <probe.asm>...
# Per probe: prints "== <probe> ASL_EXIT=<rc>" and the listing to stdout.
G=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a735fcd77afe45035/docs/superpowers/notes/asl-reference/asl_ref.sh
. "$G" || exit $?
echo "ASL_MD5=$(md5sum "$ASL" | cut -d' ' -f1) ASL=$ASL"
cd "$1" || exit 9
shift
for p in "$@"; do
    rm -f "${p%.asm}.lst" "${p%.asm}.p"
    asl_run -xx -n -q -A -L -U -i . "$p" 2> "${p%.asm}.asllog"
    rc=$?
    echo "== $p ASL_EXIT=$rc"
    /usr/bin/grep -E '^ASL_DIAG' "${p%.asm}.asllog"
done
