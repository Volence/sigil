#!/usr/bin/env bash
# Assemble every probe beside this script with the reference asl, stdout and
# stderr captured separately, three runs each for stability.
. "$(dirname "$0")/../asl-reference/asl_ref.sh" || exit $?
cd "$(dirname "$0")" || exit 2
echo "asl md5 $(md5sum "$ASL" | cut -d' ' -f1)"
for p in "$@"; do
    for i in 1 2 3; do
        asl_run -xx -n -q -A -L -U -i . "$p.asm" > "$p.stdout.$i" 2> "$p.stderr.$i"
        echo "$p run $i rc=$? $(grep -E '^ASL_DIAG' "$p.stderr.$i")"
    done
    if cmp -s "$p.stdout.1" "$p.stdout.2" && cmp -s "$p.stdout.2" "$p.stdout.3"; then
        echo "$p: stdout stable over 3 runs"
    else
        echo "$p: STDOUT UNSTABLE"
    fi
    echo "--- $p stdout ---"
    cat -A "$p.stdout.1"
    echo "--- $p stderr (asl lines) ---"
    grep -v '^ASL_' "$p.stderr.1" | cat -A
done
