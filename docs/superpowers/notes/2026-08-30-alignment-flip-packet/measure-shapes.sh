#!/usr/bin/env bash
# measure-shapes.sh <out-dir> — build all seven shapes in AEON_DIR with SIGIL_BUILD/SIGIL_EMIT,
# record CRC32+size, keep each shape's ROM + listing under <out-dir>/<shape>.{bin,lst}
# (the off-canonical shapes write listings under CANONICAL names, so each is copied out
# before the next build overwrites it), then restore the canonical s4/s4.debug artifacts.
set -uo pipefail
OUT="$1"; mkdir -p "$OUT"
: "${AEON_DIR:?}" "${SIGIL_BUILD:?}" "${SIGIL_EMIT:?}"
crc() { python3 -c "import zlib,sys;d=open(sys.argv[1],'rb').read();print(f'{zlib.crc32(d)&0xffffffff:08x}/{len(d)}')" "$1"; }

build_sh() { # <shape> <game> <rom> <lst> [DEBUG=1]
    local shape="$1" game="$2" rom="$3" lst="$4"; shift 4
    local marker; marker="$(mktemp)"; sleep 0.01
    rm -f "$AEON_DIR/$rom" "$AEON_DIR/$lst"
    ( cd "$AEON_DIR" && env "$@" NO_LINT=1 SIGIL_BUILD="$SIGIL_BUILD" SIGIL_EMIT="$SIGIL_EMIT" ./build.sh "$game" > "$OUT/$shape.build.log" 2>&1 )
    local rc=$?
    if [[ ! -f "$AEON_DIR/$rom" ]]; then echo "$shape: BUILD FAILED rc=$rc (no $rom); see $OUT/$shape.build.log"; rm -f "$marker"; return 1; fi
    [[ "$AEON_DIR/$rom" -nt "$marker" ]] || { echo "$shape: STALE (not newer than marker)"; rm -f "$marker"; return 1; }
    rm -f "$marker"
    cp "$AEON_DIR/$rom" "$OUT/$shape.bin"; cp "$AEON_DIR/$lst" "$OUT/$shape.lst"
    echo "$shape: $(crc "$OUT/$shape.bin") (build.sh rc=$rc)"
}
build_cfg() { # <shape> <flag> <rom> <lst>
    local shape="$1" flag="$2" rom="$3" lst="$4"
    local marker; marker="$(mktemp)"; sleep 0.01
    rm -f "$AEON_DIR/$rom" "$AEON_DIR/$lst"
    ( cd "$AEON_DIR" && SIGIL_EMIT="$SIGIL_EMIT" "$SIGIL_BUILD" build --aeon . --native "$flag" -o "$rom" --emit-lst "$lst" > "$OUT/$shape.build.log" 2>&1 )
    local rc=$?
    if [[ ! -f "$AEON_DIR/$rom" ]]; then echo "$shape: BUILD FAILED rc=$rc (no $rom); see $OUT/$shape.build.log"; rm -f "$marker"; return 1; fi
    [[ "$AEON_DIR/$rom" -nt "$marker" ]] || { echo "$shape: STALE"; rm -f "$marker"; return 1; }
    rm -f "$marker"
    cp "$AEON_DIR/$rom" "$OUT/$shape.bin"; cp "$AEON_DIR/$lst" "$OUT/$shape.lst"
    echo "$shape: $(crc "$OUT/$shape.bin") (sigil build rc=$rc)"
}

echo "### pwd=$PWD sigil_head=$(git rev-parse HEAD) branch=$(git branch --show-current) aeon=$AEON_DIR aeon_head=$(git -C "$AEON_DIR" rev-parse HEAD) sigil_bin=$SIGIL_BUILD md5=$(md5sum "$SIGIL_BUILD" | cut -c1-12) at=$(date -u +%FT%TZ)"
build_sh s4        sonic4 s4.bin        s4.lst
build_sh s4_debug  sonic4 s4.debug.bin  s4.debug.lst  DEBUG=1
build_sh demo      demo   demo.bin      demo.lst
build_sh demo_debug demo  demo.debug.bin demo.debug.lst DEBUG=1
build_cfg config_a --config-a s4.debug.bin s4.debug.lst
build_cfg config_b --config-b s4.bin      s4.lst
build_cfg lean     --lean     s4.bin      s4.lst
# restore canonical (both listings + ROMs cleared first so a leftover cannot pose as output)
rm -f "$AEON_DIR/s4.bin" "$AEON_DIR/s4.debug.bin" "$AEON_DIR/s4.lst" "$AEON_DIR/s4.debug.lst"
( cd "$AEON_DIR" && NO_LINT=1 SIGIL_BUILD="$SIGIL_BUILD" SIGIL_EMIT="$SIGIL_EMIT" ./build.sh sonic4 > "$OUT/restore-s4.log" 2>&1 ); echo "restore s4 rc=$? $(crc "$AEON_DIR/s4.bin" 2>/dev/null)"
( cd "$AEON_DIR" && DEBUG=1 NO_LINT=1 SIGIL_BUILD="$SIGIL_BUILD" SIGIL_EMIT="$SIGIL_EMIT" ./build.sh sonic4 > "$OUT/restore-s4_debug.log" 2>&1 ); echo "restore s4_debug rc=$? $(crc "$AEON_DIR/s4.debug.bin" 2>/dev/null)"
echo "--- abs.w ceiling falsifier (s4 / s4_debug listings as captured)"
for s in s4 s4_debug; do echo "[$s]"; grep -nE 'SoundTablesZ80_Head|Sound_PlaySFX' "$OUT/$s.lst"; done
echo "### done at=$(date -u +%FT%TZ)"
