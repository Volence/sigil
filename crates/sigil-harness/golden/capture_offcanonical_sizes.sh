#!/bin/bash
# capture_offcanonical_sizes.sh — freeze the PERISHABLE asl-derived region-boundary
# addresses for the off-canonical targets (Flip Stage 2 · S1.2, overseer condition 1).
#
# The declared-order computed-base chainer reproduces a target's frozen golden by
# reserving each emp region's EXACT asl per-region SIZE (its span). For canonical
# sonic4 those sizes come from a pinned pass-1 resolve (baked == asl-correct). For the
# OFF-CANONICAL targets (demo plain/debug, Config-A, Config-B) the baked resume orgs
# are WRONG sonic4 values, so their sizes must come from their own asl listing — which
# EXISTS ONLY WHILE ASL IS LIVE (post-flip there is no demo.lst/config .lst to parse).
# This script captures those boundary addresses NOW and commits them as frozen
# constants, exactly the anti-stale discipline of capture_goldens.sh:
#   (1) delete the target, (2) rebuild via the POSITIONAL game arg, (3) ASSERT the
#   artifact reappeared newer than a pre-build marker before reading it.
#
# Each committed table records the golden CRC it reproduces (provenance). They are the
# LAST asl-derived constants in the system; see the handoff note for the post-flip
# re-derivation path (a ruled golden re-baseline re-derives them from sigil's OWN
# layout — nothing ever requires asl again).
#
# Config-A writes s4.debug.bin and Config-B writes s4.bin (CLOBBERING the canonical
# references), so those are captured into distinct tables and the canonical refs are
# rebuilt at the end — same as capture_goldens.sh.
#
# Usage:
#   SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
#   AEON_DIR=/path/to/aeon ./capture_offcanonical_sizes.sh
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SIGIL_ROOT="$(cd "$HERE/../../.." && pwd)"
AEON="${AEON_DIR:-/home/volence/sonic_hacks/aeon}"
[[ -d "$AEON" ]] || { echo "ERROR: AEON_DIR not a dir: $AEON"; exit 1; }
if [[ -z "${SIGIL_EMIT:-}" || ! -x "${SIGIL_EMIT:-}" ]]; then
    echo "ERROR: set SIGIL_EMIT to <sigil>/target/release/emit_sound_blob (sound-on builds need it)."
    exit 1
fi
CAP="$SIGIL_ROOT/target/release/capture_offcanon"
[[ -x "$CAP" ]] || { echo "ERROR: build the capture binary first: cargo build --release -p sigil-harness --bin capture_offcanon"; exit 1; }
OUT="$HERE/offcanonical_sizes"
mkdir -p "$OUT"

# capture <target> <game> <out_rom> <out_lst> <env...>
capture() {
    local target="$1" game="$2" rom="$3" lst="$4"; shift 4
    local path="$AEON/$rom" lstp="$AEON/$lst"
    local marker; marker="$(mktemp)"; sleep 0.01
    rm -f "$path" "$lstp"
    echo ">> $target  ($game: ${*:-canonical})"
    ( cd "$AEON" && env "$@" SIGIL_EMIT="$SIGIL_EMIT" ./build.sh "$game" >/dev/null )
    [[ -f "$path" && -f "$lstp" ]] || { echo "FAIL: $rom / $lst not produced"; rm -f "$marker"; exit 1; }
    [[ "$path" -nt "$marker" ]] || { echo "FAIL: $rom not newer than the pre-build marker (stale-capture guard)"; rm -f "$marker"; exit 1; }
    rm -f "$marker"
    "$CAP" "$target" "$lstp" "$path" "$OUT/$target.txt"
}

echo "== Flip Stage 2 off-canonical size capture (perishable — asl-live only) =="
echo "   aeon: $AEON  ($(cd "$AEON" && git rev-parse --short HEAD 2>/dev/null || echo '?'))"

capture demo        demo   demo.bin        demo.lst
capture demo_debug  demo   demo.debug.bin  demo.debug.lst  DEBUG=1
# Config-A/B CLOBBER s4.debug.bin / s4.bin (captured into distinct tables; restored below).
capture config_a    sonic4 s4.debug.bin    s4.debug.lst    DEBUG=1 SOUND_DRIVER_ENABLED=1 SOUND_DEBUG_HOTKEYS=1 SOUND_DBG_MIRROR=1
capture config_b    sonic4 s4.bin          s4.lst          SOUND_DRIVER_ENABLED=0

echo ">> restoring canonical aeon s4.bin + s4.debug.bin ..."
( cd "$AEON" && SIGIL_EMIT="$SIGIL_EMIT" ./build.sh sonic4 >/dev/null && DEBUG=1 SIGIL_EMIT="$SIGIL_EMIT" ./build.sh sonic4 >/dev/null )
echo "   restored: $(cd "$AEON" && python3 -c "import zlib;print('s4.bin',f'{zlib.crc32(open(\"s4.bin\",\"rb\").read())&0xffffffff:08x}','/','s4.debug.bin',f'{zlib.crc32(open(\"s4.debug.bin\",\"rb\").read())&0xffffffff:08x}')")"
echo "== done — tables in $OUT (commit them; they are perishable golden provenance) =="
