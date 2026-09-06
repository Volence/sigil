#!/usr/bin/env bash
# corpus-prepare.sh - run a corpus disassembly's OWN generator half in a tree,
# so the tree holds the build-time generated include files the assembly sources
# ask for.
#
# A bare `git` checkout of s2disasm or s1disasm has no `*/generated/*.inc`: the
# corpus's `build.lua` writes those from `.wav` and `.asm` sources at build
# time, and they are gitignored. An assembler run over a bare checkout therefore
# counts the assembler's own defects PLUS an absent generator's output and
# cannot tell them apart. This script closes that gap.
#
# Usage: scripts/corpus-prepare.sh <corpus-dir>
#
# The cut point between the generator half and the ROM build is DERIVED from
# build.lua by pattern, never by line number: everything before the first call
# to `build_rom_and_handle_failure` generates input, everything from it on
# assembles and post-processes a ROM. The cut line is printed so a reader can
# check the derivation rather than trust it.
#
# Exit status: 0 when the generator ran and wrote files; nonzero otherwise. A
# run that writes ZERO files is a FAILURE, not a no-op: it is exactly what a
# silently broken generator looks like.
set -u

CORPUS="${1:-}"
if [ -z "$CORPUS" ] || [ ! -d "$CORPUS" ]; then
    echo "FATAL: usage: scripts/corpus-prepare.sh <corpus-dir>" >&2
    exit 2
fi
CORPUS="$(cd "$CORPUS" && pwd)"

if [ ! -f "$CORPUS/build.lua" ]; then
    echo "FATAL: $CORPUS has no build.lua, so there is no generator half to run" >&2
    exit 2
fi

command -v lua >/dev/null 2>&1 || {
    echo "FATAL: no 'lua' on PATH; the corpus generator is a Lua script" >&2
    exit 2
}

echo "== corpus =="
echo "  dir       $CORPUS"
echo "  rev       $(cd "$CORPUS" && git rev-parse HEAD 2>/dev/null || echo '(not a git tree)')"
echo "  dirty     $(cd "$CORPUS" && git status --porcelain 2>/dev/null | wc -l) path(s)"
echo "  lua       $(lua -v 2>&1 | head -1)"

# ---------------------------------------------------------------------------
# Derive the cut point.
# ---------------------------------------------------------------------------
BUILD_LINES=$(wc -l < "$CORPUS/build.lua")
CUT=$(grep -n 'build_rom_and_handle_failure' "$CORPUS/build.lua" | head -1 | cut -d: -f1)
if [ -z "$CUT" ]; then
    echo "FATAL: build.lua names no 'build_rom_and_handle_failure', so the boundary" >&2
    echo "       between generating input and building a ROM cannot be derived." >&2
    echo "       Refusing to guess a line number." >&2
    exit 3
fi
KEEP=$((CUT - 1))
if [ "$KEEP" -lt 1 ]; then
    echo "FATAL: the ROM build is build.lua's first line; there is no generator half" >&2
    exit 3
fi

echo
echo "== derived cut =="
echo "  build.lua is $BUILD_LINES line(s); the ROM build is line $CUT:"
sed -n "${CUT}p" "$CORPUS/build.lua" | sed 's/^/      /'
echo "  keeping lines 1..$KEEP as the generator half"

GEN_LUA="$CORPUS/.sigil-corpus-generate.lua"
cleanup() { rm -f "$GEN_LUA"; }
trap cleanup EXIT
head -n "$KEEP" "$CORPUS/build.lua" > "$GEN_LUA"

# ---------------------------------------------------------------------------
# Count what is there before, so "wrote files" is a measurement.
# ---------------------------------------------------------------------------
count_generated() {
    find "$CORPUS" -path '*/generated/*' -type f \
        ! -name 'hashes.lua' ! -name '*will be*generated here' 2>/dev/null | wc -l
}
BEFORE_N=$(count_generated)
echo "  generated files already present: $BEFORE_N"

# ---------------------------------------------------------------------------
# Run it.
# ---------------------------------------------------------------------------
echo
echo "== generator run =="
( cd "$CORPUS" && lua .sigil-corpus-generate.lua ) 2>&1 | sed 's/^/  /'
RC=${PIPESTATUS[0]}
echo "  generator exit=$RC"

AFTER_N=$(count_generated)
echo "  generated files now present: $AFTER_N (was $BEFORE_N, wrote $((AFTER_N - BEFORE_N)))"

if [ "$RC" -ne 0 ]; then
    echo "FAIL: the generator exited $RC; the tree is not prepared" >&2
    exit "$RC"
fi
if [ "$AFTER_N" -eq 0 ]; then
    echo "FAIL: the generator exited 0 and wrote NOTHING. An empty generated set is" >&2
    echo "      indistinguishable from a generator that never ran, so this is a" >&2
    echo "      failure and not a clean no-op." >&2
    exit 4
fi

echo
echo "== what was written =="
find "$CORPUS" -path '*/generated/*' -type f ! -name 'hashes.lua' \
    ! -name '*will be*generated here' -printf '%P\n' 2>/dev/null | sort | sed 's/^/  /'

echo
echo "CORPUS-PREPARE-END rc=0 files=$AFTER_N"
