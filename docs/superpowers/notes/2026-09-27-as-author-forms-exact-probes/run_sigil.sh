#!/usr/bin/env bash
# Assembles every probe here through sigil's AS route (`sigil <probe>.asm -o
# <probe>.bin`) and prints exit status and bytes, in the same shape as
# run_asl.sh so the two outputs diff line for line. $SIGIL names the binary.
cd "$(dirname "$0")" || exit 2
: "${SIGIL:?set SIGIL to the sigil binary under test}"
OUT=$(mktemp -d ./.out.XXXX)
for f in *.asm; do
    n=${f%.asm}
    if "$SIGIL" "$f" -o "$OUT/$n.bin" > "$OUT/$n.log" 2>&1; then
        printf '%-16s exit 0   %s\n' "$n" "$(od -An -tx1 -v "$OUT/$n.bin" | tr -d ' \n' | tr a-f A-F)"
    else
        printf '%-16s exit %s  REFUSED  %s\n' "$n" "$?" "$(grep -m1 -i 'error' "$OUT/$n.log" | cut -c1-140)"
    fi
done
rm -rf "$OUT"
