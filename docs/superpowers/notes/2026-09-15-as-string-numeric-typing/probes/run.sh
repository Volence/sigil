#!/usr/bin/env bash
# Run one probe through the pinned reference asl and print its listing body.
. /home/volence/sonic_hacks/sigil/docs/superpowers/notes/asl-reference/asl_ref.sh || exit $?
cd "$(dirname "$0")" || exit 2
for f in "$@"; do
    echo "===== $f ====="
    rm -f "${f%%.*}.lst" "${f%%.*}.p"
    asl_run -xx -n -q -A -L -U -i . "$f"
    echo "RC=$?"
    sed -n '/Source File/,/Symbol Table/p' "${f%%.*}.lst" | sed '/Symbol Table/,$d'
done
