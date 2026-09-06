#!/usr/bin/env bash
# Run the PINNED reference asl over a prepared corpus tree and stamp the run.
#
# usage: run_asl.sh <corpus-dir> <entry.asm> <out-prefix> [extra asl args...]
#
# The guard beside this file selects the reference build by md5 and refuses any
# other, and `asl_run` reports the exit status and classifies the listing footer.
# Both matter here: the s2disasm build answers differently between runs for an
# operand it declines to value, and a run carrying ANY error abandons its pass
# loop, so its byte column is not a source of values.
#
# Writes <out-prefix>.out and <out-prefix>.err. The listing lands beside the
# source inside the corpus, named by asl's own rule (source path truncated at
# its FIRST dot), which is why the entry is passed relative from the corpus dir.
set -u
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CORPUS="${1:-}"; ENTRY="${2:-}"; OUT="${3:-}"
if [ -z "$CORPUS" ] || [ -z "$ENTRY" ] || [ -z "$OUT" ]; then
    echo "usage: run_asl.sh <corpus-dir> <entry.asm> <out-prefix> [extra asl args...]" >&2
    exit 2
fi
shift 3
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
cd "$CORPUS" || exit 2
asl_run -xx -n -q -A -L -U -i . "$@" "$ENTRY" > "$OUT.out" 2> "$OUT.err"
RC=$?
echo "ASL_RC=$RC"
echo "  corpus $CORPUS"
echo "  entry  $ENTRY"
echo "  extra  $*"
exit 0
