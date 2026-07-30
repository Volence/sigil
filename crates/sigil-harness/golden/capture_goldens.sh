#!/bin/bash
# capture_goldens.sh — the ONE-STEP fresh-build golden capture (Flip Stage 1 · S1.6).
#
# Captures ALL SIX Stage-2 comparand ROMs asl-derived WHILE ASL IS LIVE — the only
# moment the off-canonical Config-A/B are reproducible (they have no shipped file and
# their NATIVE reproduction is Stage-2-coupled; see PROVENANCE.md + the
# 2026-07-30-flip-stage1-demo-config-native-blocked.md fork note). For each target it
# records BOTH split-golden layers: the FULL FILE (assembled + convsym deb2 appendix)
# and the assembled-ROM ANCHOR (0..EndOfRom, header-neutral — the PRIMARY-class CRC,
# the drift-stable Stage-2 bar).
#
# It is STRUCTURALLY IMPOSSIBLE to capture a stale artifact: for each ROM the script
# (1) deletes the target, (2) rebuilds via the POSITIONAL game arg (`./build.sh <game>`
# / env-prefixed — NEVER `GAME=<game> ./build.sh`, which build.sh IGNORES:
# `GAME="${1:-sonic4}"` is positional, build.sh:4), and (3) ASSERTS the artifact
# reappeared newer than a pre-build marker (a real rebuild) before CRC-ing it. This
# guard catches the demo stale-baseline class: the false `2b71b37d/88738` plain-demo
# "golden" was a pre-existing demo.bin CRC'd WITHOUT a rebuild (compounded by the
# ignored `GAME=` env var); the TRUE plain demo bar is `18c64002/90776`.
#
# Config-A (DEBUG+hotkeys+mirror) writes to s4.debug.bin and Config-B (sound-off)
# writes to s4.bin — so they CLOBBER the canonical references. This script captures
# the four canonical/demo goldens FIRST, then Config-A/B into distinct golden blobs
# (config_a.bin / config_b.bin), then REBUILDS canonical s4.bin + s4.debug.bin so the
# aeon tree is left in its canonical state for every other gate that reads them.
#
# Usage:
#   SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
#   AEON_DIR=/path/to/aeon ./capture_goldens.sh [--write]
#     --write  copy each fresh full file into this golden dir as a committed blob.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
AEON="${AEON_DIR:-/home/volence/sonic_hacks/aeon}"
WRITE=0
[[ "${1:-}" == "--write" ]] && WRITE=1
[[ -d "$AEON" ]] || { echo "ERROR: AEON_DIR not a dir: $AEON"; exit 1; }
if [[ -z "${SIGIL_EMIT:-}" || ! -x "${SIGIL_EMIT:-}" ]]; then
    echo "ERROR: set SIGIL_EMIT to <sigil>/target/release/emit_sound_blob (sound-on builds need it)."
    exit 1
fi

# report <lst> <bin> — print both split-golden layers (full file + header-neutral
# assembled anchor). EndOfRom (the appendix boundary) is read from the .lst END line.
report() {
    python3 - "$1" "$2" <<'PY'
import zlib, re, sys
lst, binf = sys.argv[1], sys.argv[2]
end = None
for line in open(lst, errors='replace'):
    m = re.search(r'/\s*([0-9A-Fa-f]+)\s*:\s+END\s*$', line)
    if m:
        end = int(m.group(1), 16)  # last match wins
raw = open(binf, 'rb').read()
if end is None or end > len(raw):
    print(f"FAIL: bad EndOfRom {end} for {binf} (len {len(raw)})"); sys.exit(1)
# header-neutral: zero the checksum ($18E..$190) and ROM-end pointer ($1A4..$1A8)
d = bytearray(raw)
for i in range(0x18E, 0x190): d[i] = 0
for i in range(0x1A4, 0x1A8): d[i] = 0
full_crc = zlib.crc32(raw) & 0xffffffff
anc_crc  = zlib.crc32(bytes(d[:end])) & 0xffffffff
print(f"full {full_crc:08x} / {len(raw)}    anchor {anc_crc:08x} / {end}  (EndOfRom {end:#x})")
PY
}

# capture <golden_name> <game> <out_rom> <out_lst> <env...>
capture() {
    local golden="$1" game="$2" rom="$3" lst="$4"; shift 4
    local path="$AEON/$rom" lstp="$AEON/$lst"
    local marker; marker="$(mktemp)"; sleep 0.01
    rm -f "$path"
    echo ">> $golden  ($game: ${*:-canonical})"
    ( cd "$AEON" && env "$@" SIGIL_EMIT="$SIGIL_EMIT" ./build.sh "$game" >/dev/null )
    [[ -f "$path" ]] || { echo "FAIL: $rom not produced"; rm -f "$marker"; exit 1; }
    [[ "$path" -nt "$marker" ]] || { echo "FAIL: $rom not newer than the pre-build marker (stale-capture guard)"; rm -f "$marker"; exit 1; }
    rm -f "$marker"
    printf "   "; report "$lstp" "$path"
    if [[ "$WRITE" == "1" ]]; then cp "$path" "$HERE/$golden"; echo "   frozen -> golden/$golden"; fi
}

echo "== Flip Stage 1 golden capture — all six, fresh-build, asl-derived =="
echo "   aeon: $AEON  ($(cd "$AEON" && git rev-parse --short HEAD 2>/dev/null || echo '?'))"

# 1-4: the canonical + demo goldens (each writes its own filename; no clobber).
capture s4.bin        sonic4 s4.bin        s4.lst
capture s4.debug.bin  sonic4 s4.debug.bin  s4.debug.lst  DEBUG=1
capture demo.bin      demo   demo.bin      demo.lst
capture demo.debug.bin demo  demo.debug.bin demo.debug.lst DEBUG=1

# 5-6: the off-canonical configs (CLOBBER s4.debug.bin / s4.bin — captured into
# distinct golden blobs; canonical is rebuilt below).
capture config_a.bin  sonic4 s4.debug.bin  s4.debug.lst  DEBUG=1 SOUND_DRIVER_ENABLED=1 SOUND_DEBUG_HOTKEYS=1 SOUND_DBG_MIRROR=1
capture config_b.bin  sonic4 s4.bin        s4.lst        SOUND_DRIVER_ENABLED=0

# RESTORE the canonical aeon references clobbered by Config-A/B.
echo ">> restoring canonical aeon s4.bin + s4.debug.bin ..."
( cd "$AEON" && SIGIL_EMIT="$SIGIL_EMIT" ./build.sh sonic4 >/dev/null && DEBUG=1 SIGIL_EMIT="$SIGIL_EMIT" ./build.sh sonic4 >/dev/null )
echo "   restored: $(cd "$AEON" && python3 -c "import zlib;print('s4.bin',f'{zlib.crc32(open(\"s4.bin\",\"rb\").read())&0xffffffff:08x}','/','s4.debug.bin',f'{zlib.crc32(open(\"s4.debug.bin\",\"rb\").read())&0xffffffff:08x}')")"

echo "== done — expected full-file bars (aeon bcb8f64; see PROVENANCE.md) =="
echo "   s4.bin eff2396f/413577  s4.debug.bin 1e9097bc/421579  demo.bin 18c64002/90776  demo.debug.bin b0475a59/91584"
echo "   config_a.bin b4a6756d/421898  config_b.bin 92776720/304961"
