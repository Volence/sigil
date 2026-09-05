#!/usr/bin/env bash
# Build all four aeon shapes with one sigil binary and print CRC32+size per shape.
# The landing condition is this table being identical between the branch-point
# binary and the parcel's, so the script takes the binary as an argument and the
# two runs are the same command twice.
#
#   ./fourshapes.sh <sigil-binary> <aeon-dir> <out-dir>
set -uo pipefail
SIGIL="$1"; AEON="$2"; OUT="$3"
mkdir -p "$OUT"
export AEON_DIR="$AEON"
for shape in "sonic4:" "sonic4:--debug" "demo:" "demo:--debug"; do
    g="${shape%%:*}"; d="${shape##*:}"
    tag="$g${d:+-debug}"
    # shellcheck disable=SC2086
    "$SIGIL" build --aeon "$AEON" --game "$g" $d -o "$OUT/$tag.bin" >"$OUT/$tag.log" 2>&1
    rc=$?
    if [[ $rc -ne 0 ]]; then
        echo "$tag BUILD_EXIT=$rc  --- REFUSED, first lines:"
        grep -i 'error' "$OUT/$tag.log" | head -5
        continue
    fi
    printf '%s\tBUILD_EXIT=%d\tcrc32=%s\tsize=%s\n' \
        "$tag" "$rc" \
        "$(python3 -c 'import sys,zlib;print("%08x"%(zlib.crc32(open(sys.argv[1],"rb").read())&0xffffffff))' "$OUT/$tag.bin")" \
        "$(stat -c%s "$OUT/$tag.bin")"
done
