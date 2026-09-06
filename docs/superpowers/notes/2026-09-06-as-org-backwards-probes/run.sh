#!/usr/bin/env bash
# Run one probe beside this file through the reference asl and through a sigil
# binary, and print both answers side by side.
#
# usage: run.sh <probe.asm> <sigil-binary>
#
# These are PROBES, not a gate. They carry no verdict and nothing here runs in a
# suite. What they establish is written in the note that names this directory;
# each probe's own header says what it asks and which answers would refute the
# model it was built to test.
#
# The asl guard beside `asl-reference/` selects the reference build by md5 and
# refuses any other, and `asl_run` reports the exit status and classifies the
# listing footer. Both matter: one of the four asl builds in this workspace
# answers differently between runs for an operand it declines to value, and a run
# carrying ANY error abandons its pass loop, so its byte column is not a source
# of values.
#
# asl names its listing after the source path TRUNCATED AT THE FIRST DOT, so this
# copies the probe into a work directory with no dot-component in its path and
# assembles from there. Handing asl a path through `.claude/worktrees/...` writes
# the listing at the repository root and reads back a stale or missing one.
set -u
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROBE="${1:-}"
SIGIL="${2:-}"
if [ -z "$PROBE" ] || [ -z "$SIGIL" ]; then
    echo "usage: run.sh <probe.asm> <sigil-binary>" >&2
    exit 2
fi
[ -f "$PROBE" ] || { echo "FATAL: no probe at $PROBE" >&2; exit 2; }
[ -x "$SIGIL" ] || { echo "FATAL: $SIGIL is not an executable" >&2; exit 2; }
PROBE="$(cd "$(dirname "$PROBE")" && pwd)/$(basename "$PROBE")"
SIGIL="$(cd "$(dirname "$SIGIL")" && pwd)/$(basename "$SIGIL")"

. "$HERE/../asl-reference/asl_ref.sh" || exit $?

WORK="$(mktemp -d /tmp/as_org_backwards_XXXXXX)"
cp "$PROBE" "$WORK/probe.asm"
cd "$WORK" || exit 2

echo "== probe =="
echo "  source $PROBE"
echo "  sigil  $SIGIL  md5 $(md5sum "$SIGIL" | cut -d' ' -f1)"
echo "  asl    $ASL  md5 $ASL_REF_MD5"
echo "  work   $WORK"

echo
echo "== asl =="
asl_run -xx -n -q -A -L -U -i . -o probe.p probe.asm
ASL_RC=$?
echo "ASL_RC=$ASL_RC"
if [ "$ASL_RC" -eq 0 ] && [ -f "$ASLDIR/p2bin" ]; then
    "$ASLDIR/p2bin" probe.p asl.bin >/dev/null 2>&1
    if [ -f asl.bin ]; then
        echo "  image $(stat -c %s asl.bin) byte(s), md5 $(md5sum asl.bin | cut -d' ' -f1)"
    fi
fi

echo
echo "== sigil =="
"$SIGIL" probe.asm -o sigil.bin
echo "SIGIL_RC=$?"
if [ -f sigil.bin ]; then
    echo "  image $(stat -c %s sigil.bin) byte(s), md5 $(md5sum sigil.bin | cut -d' ' -f1)"
fi

echo
if [ -f asl.bin ] && [ -f sigil.bin ]; then
    if cmp -s asl.bin sigil.bin; then
        echo "IMAGES IDENTICAL"
    else
        echo "IMAGES DIFFER"
        cmp -l asl.bin sigil.bin | head -20
    fi
else
    echo "NO IMAGE COMPARISON: one side produced no binary (see the statuses above)."
fi
