#!/bin/bash
# Build all four aeon shapes with a named sigil toolchain; report CRC32 + size.
# One shape per invocation of build.sh. A ROM is removed before its own build,
# so a PRESENT file afterwards is necessarily this run's.
set -u
TD="$1"; TAG="$2"; OUT="$3"
SB="$TD/release/sigil"; SE="$TD/release/emit_sound_blob"
cd /home/volence/sonic_hacks/.aeon-f4ref || exit 9
: > "$OUT"
START=$(date +%s)
for spec in "sonic4:plain:s4.bin" "sonic4:debug:s4.debug.bin" "demo:plain:demo.bin" "demo:debug:demo.debug.bin"; do
  GAME="${spec%%:*}"; rest="${spec#*:}"; MODE="${rest%%:*}"; ROM="${rest#*:}"
  rm -f "$ROM"
  LOG="/tmp/aeonlog.$TAG.$GAME.$MODE.txt"
  if [ "$MODE" = debug ]; then
    DEBUG=1 SIGIL_BUILD="$SB" SIGIL_EMIT="$SE" ./build.sh "$GAME" > "$LOG" 2>&1
  else
    SIGIL_BUILD="$SB" SIGIL_EMIT="$SE" ./build.sh "$GAME" > "$LOG" 2>&1
  fi
  ec=$?
  if [ -f "$ROM" ]; then
    MT=$(stat -c %Y "$ROM")
    if [ "$MT" -lt "$START" ]; then FRESH="STALE-PREDATES-RUN"; else FRESH="fresh"; fi
    printf '%s %-6s %-5s exit=%s crc32=%s size=%s %s\n' "$TAG" "$GAME" "$MODE" "$ec" \
      "$(python3 -c "import zlib,sys;print('%08x'%(zlib.crc32(open(sys.argv[1],'rb').read())&0xffffffff))" "$ROM")" \
      "$(stat -c %s "$ROM")" "$FRESH" >> "$OUT"
  else
    printf '%s %-6s %-5s exit=%s ROM-ABSENT (DID NOT BUILD)\n' "$TAG" "$GAME" "$MODE" "$ec" >> "$OUT"
  fi
done
echo "AEON4-END-$TAG" >> "$OUT"
