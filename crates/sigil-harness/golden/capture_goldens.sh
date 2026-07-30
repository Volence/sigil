#!/bin/bash
# capture_goldens.sh — the ONE-STEP fresh-build golden capture (Flip Stage 1 · S1.6).
#
# It is STRUCTURALLY IMPOSSIBLE to capture a stale artifact here: for each ROM the
# script (1) deletes the target, (2) rebuilds it via the POSITIONAL game arg
# (`./build.sh <game>` / `DEBUG=1 ./build.sh <game>` — NEVER `GAME=<game> ./build.sh`,
# which build.sh IGNORES: `GAME="${1:-sonic4}"` is positional, build.sh:4), and
# (3) ASSERTS the artifact reappeared with an mtime newer than a pre-build marker
# file — a real rebuild — before CRC-ing it. This is the guard that catches the
# demo stale-baseline class: the false `2b71b37d/88738` plain-demo "golden" was a
# pre-existing demo.bin CRC'd WITHOUT a rebuild (compounded by the ignored `GAME=`
# env var); the TRUE plain demo bar is `18c64002/90776` (see PROVENANCE.md).
#
# Usage:
#   SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
#   AEON_DIR=/path/to/aeon ./capture_goldens.sh [--write]
#
#   --write  also copy each fresh ROM into this golden dir as a committed blob.
#            Without it, the script only rebuilds + reports CRC/size (verify mode).
#
# NOTE (2026-07-30): Config-A / Config-B goldens are NOT captured here — they have
# no shipped asl file and their native reproduction is BLOCKED on S1.2 (computed
# resume orgs); see docs/superpowers/notes/2026-07-30-flip-stage1-demo-config-
# native-blocked.md. This script freezes the four REAL asl-produced ROMs only.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
AEON="${AEON_DIR:-/home/volence/sonic_hacks/aeon}"
WRITE=0
[[ "${1:-}" == "--write" ]] && WRITE=1

[[ -d "$AEON" ]] || { echo "ERROR: AEON_DIR not a dir: $AEON"; exit 1; }

crc32() { python3 -c "import zlib,sys;print(f'{zlib.crc32(open(sys.argv[1],\"rb\").read())&0xffffffff:08x}')" "$1"; }

# capture <game> <shape:plain|debug> <rom_filename>
capture() {
    local game="$1" shape="$2" rom="$3"
    local path="$AEON/$rom"
    local marker; marker="$(mktemp)"           # a fresh mtime floor
    sleep 0.01                                  # ensure the rebuild's mtime can exceed it
    rm -f "$path"                               # force a genuine rebuild
    echo ">> building $rom ($game, $shape) ..."
    (
        cd "$AEON"
        if [[ "$shape" == "debug" ]]; then
            DEBUG=1 SIGIL_EMIT="${SIGIL_EMIT:-}" ./build.sh "$game" >/dev/null
        else
            SIGIL_EMIT="${SIGIL_EMIT:-}" ./build.sh "$game" >/dev/null
        fi
    )
    # STRUCTURAL anti-stale assertion: the artifact must exist AND be newer than the
    # marker (a real rebuild happened this invocation). A pre-existing/stale file
    # cannot satisfy `-nt` against a marker minted this run.
    [[ -f "$path" ]] || { echo "FAIL: $rom was not produced by the build"; rm -f "$marker"; exit 1; }
    [[ "$path" -nt "$marker" ]] || {
        echo "FAIL: $rom is NOT newer than the pre-build marker — the build did not"
        echo "      regenerate it (stale-capture guard tripped). Refusing to freeze."
        rm -f "$marker"; exit 1
    }
    rm -f "$marker"
    local size crc; size="$(stat -c %s "$path")"; crc="$(crc32 "$path")"
    printf "   %-16s %8s bytes  crc32 %s\n" "$rom" "$size" "$crc"
    if [[ "$WRITE" == "1" ]]; then
        cp "$path" "$HERE/$rom"
        echo "   frozen -> golden/$rom"
    fi
}

echo "== Flip Stage 1 golden capture (fresh-build) =="
echo "   aeon: $AEON  ($(cd "$AEON" && git rev-parse --short HEAD 2>/dev/null || echo '?'))"
# sonic4 is sound-ON → build.sh REQUIRES SIGIL_EMIT (the resident sound blob has no
# asl fallback). demo is sound-OFF → SIGIL_EMIT is unused (harmless if set).
if [[ -z "${SIGIL_EMIT:-}" || ! -x "${SIGIL_EMIT:-}" ]]; then
    echo "ERROR: set SIGIL_EMIT to <sigil>/target/release/emit_sound_blob (sonic4 needs it)."
    exit 1
fi

capture sonic4 plain s4.bin
capture sonic4 debug s4.debug.bin
capture demo   plain demo.bin
capture demo   debug demo.debug.bin

echo "== done =="
echo "Expected (aeon bcb8f64 — the frozen bars; see PROVENANCE.md):"
echo "   s4.bin         eff2396f / 413577    s4.debug.bin   1e9097bc / 421579"
echo "   demo.bin       18c64002 / 90776     demo.debug.bin b0475a59 / 91584"
