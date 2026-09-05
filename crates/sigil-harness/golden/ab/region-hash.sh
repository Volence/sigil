#!/bin/bash
# region-hash.sh — the CLIENT-SIDE region hash for oracle A/B evidence (§17 arc).
#
# The oracle emulator is driven from the agent via MCP (`emulator_read_memory` /
# `emulator_screenshot`), so the A/B "runner" is the agent following AB_PROTOCOL.md.
# This helper is the one mechanical piece: it hashes a captured region file with the
# campaign's crc32+size standard AND enforces the standing no-hand-transcribe rule (a
# length assert catches a dropped/dupe byte before it can masquerade as a data diff).
#
# A region capture is a RAW BYTE file. Save `emulator_read_memory` output as raw bytes
# (or a screenshot PNG straight to disk — those `cmp` directly, no hashing needed).
#
# Usage:
#   region-hash.sh <file> [expected_bytes]      # print "crc32 / bytes"; assert length
#   region-hash.sh --diff <old> <new>           # IDENTICAL / DIFFER (first-diff offset)
set -euo pipefail

if [[ "${1:-}" == "--diff" ]]; then
    old="${2:?old file}"; new="${3:?new file}"
    if cmp -s "$old" "$new"; then
        echo "IDENTICAL  ($(wc -c <"$old") bytes)"
        exit 0
    fi
    off="$(cmp "$old" "$new" 2>/dev/null | sed -n 's/.* differ: byte \([0-9]*\),.*/\1/p' || true)"
    echo "DIFFER  first-diff byte ${off:-?}  (old $(wc -c <"$old")B / new $(wc -c <"$new")B)"
    exit 1
fi

file="${1:?capture file}"
want="${2:-}"
n="$(wc -c <"$file")"
if [[ -n "$want" && "$n" != "$want" ]]; then
    echo "LENGTH FAIL: $file is ${n}B, expected ${want}B (dropped/dupe capture, reject this evidence)" >&2
    exit 2
fi
crc="$(python3 -c "import zlib,sys;print(f'{zlib.crc32(open(sys.argv[1],\"rb\").read())&0xffffffff:08x}')" "$file")"
echo "$crc / $n"
