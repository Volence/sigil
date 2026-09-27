#!/usr/bin/env bash
# Run mkprobes.sh first: it writes the probe sources this reads.
# Assembles every probe here through the pinned reference asl (`asl_run`,
# md5 61e67256...) and prints, per probe, the exit status and the bytes.
# Bytes are read ONLY from exit-0 runs; a non-zero run prints REFUSED and the
# first error line, never a byte.
#
#   asl_run -xx -n -q -A -L -U -i .. <probe>.asm ; p2bin <probe>.p <probe>.bin
# (run from a scratch subdirectory holding a copy of the probe, so -i .. is
# this directory, where the binclude probes find their files)
cd "$(dirname "$0")" || exit 2
. ../asl-reference/asl_ref.sh || exit $?
P2BIN=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/p2bin
OUT=$(mktemp -d ./.out.XXXX)
for f in *.asm; do
    n=${f%.asm}
    cp "$f" "$OUT/$f"
    ( cd "$OUT" && asl_run -xx -n -q -A -L -U -i .. "$f" ) > "$OUT/$n.log" 2>&1
    rc=$(grep -o 'ASL_EXIT=[0-9]*' "$OUT/$n.log" | cut -d= -f2)
    if [ "$rc" = 0 ]; then
        "$P2BIN" "$OUT/$n.p" "$OUT/$n.bin" > /dev/null 2>&1
        printf '%-16s exit 0   %s\n' "$n" "$(od -An -tx1 -v "$OUT/$n.bin" | tr -d ' \n' | tr a-f A-F)"
    else
        printf '%-16s exit %s  REFUSED  %s\n' "$n" "$rc" "$(grep -m1 -i 'error' "$OUT/$n.lst" "$OUT/$n.log" 2>/dev/null | head -1 | cut -c1-140)"
    fi
done
rm -rf "$OUT"
