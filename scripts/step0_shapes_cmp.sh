#!/usr/bin/env bash
# Byte-neutral proof for the step-0 soundness plan: build the SAME aeon tree with a
# BASE and a CANDIDATE sigil in 7 shapes and compare the ROM bytes.
#
#   scripts/step0_shapes_cmp.sh --base <sigil> --cand <sigil> --aeon-ref <dir>
#
# Exit 0 = all 7 identical, 1 = at least one differs, 2 = could not run (a failed
# build or missing file is NEVER a pass). Output: one
#   SHAPE <name> <crc32>/<size> <crc32>/<size> SAME|DIFFER
# row per shape, then `SHAPES <k>/7 IDENTICAL`, then `STEP0_CMP_END rc=<n>`.
#
# A `sigil build` from before G0b Task 6 rewrites files inside the tree it builds (Sigil
# audit finding 9), and a BASE binary may be one, so each binary gets its own `cp -a` of the reference tree under a run-unique scratch
# dir. The scratch goes ON DISK in the worktree (.scratch-cmp-*), not /tmp (RAM), and
# is removed at the end of every run.
#
# The shape list is derived from `sigil build --help`:
#   [--game sonic4|demo] [--debug] [--config-a|--config-b|--lean]
set -uo pipefail

BASE="" CAND="" REF=""
while [ $# -gt 0 ]; do
  case "$1" in
    --base) BASE="${2:-}"; shift 2 ;;
    --cand) CAND="${2:-}"; shift 2 ;;
    --aeon-ref) REF="${2:-}"; shift 2 ;;
    *) echo "usage: $0 --base <sigil> --cand <sigil> --aeon-ref <dir>" >&2; echo "STEP0_CMP_END rc=2"; exit 2 ;;
  esac
done

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRATCH="$HERE/.scratch-cmp-$$-$(date +%s)"
cleanup() { rm -rf "$SCRATCH"; }
trap cleanup EXIT

die() { echo "COULD NOT RUN ${1}: ${2}"; echo "STEP0_CMP_END rc=2"; exit 2; }

for v in BASE CAND REF; do
  [ -n "${!v}" ] || die setup "missing --$(echo "$v" | tr A-Z a-z | sed 's/ref/aeon-ref/')"
done
[ -x "$BASE" ] || die setup "base binary not executable: $BASE"
[ -x "$CAND" ] || die setup "cand binary not executable: $CAND"
[ -d "$REF" ]  || die setup "aeon reference tree missing: $REF"
BASE="$(cd "$(dirname "$BASE")" && pwd)/$(basename "$BASE")"
CAND="$(cd "$(dirname "$CAND")" && pwd)/$(basename "$CAND")"

# name|flags
SHAPES=(
  "plain|"
  "debug|--debug"
  "demo|--game demo"
  "demo_debug|--game demo --debug"
  "config_a|--config-a"
  "config_b|--config-b"
  "lean|--lean"
)

mkdir -p "$SCRATCH" || die setup "cannot create $SCRATCH"
cp -a "$REF" "$SCRATCH/base-tree" || die setup "copy of reference tree for base failed"
cp -a "$REF" "$SCRATCH/cand-tree" || die setup "copy of reference tree for cand failed"

crc_size() { # file -> "<crc32 hex8>/<size>"; empty on failure
  python3 - "$1" <<'PY' 2>/dev/null
import sys, zlib
d = open(sys.argv[1], 'rb').read()
print("%08x/%d" % (zlib.crc32(d) & 0xffffffff, len(d)))
PY
}

same=0
total=${#SHAPES[@]}
differ=0
for row in "${SHAPES[@]}"; do
  name="${row%%|*}"; flags="${row#*|}"
  for side in base cand; do
    bin="$BASE"; [ "$side" = cand ] && bin="$CAND"
    out="$SCRATCH/$name.$side.bin"
    # shellcheck disable=SC2086
    "$bin" build --aeon "$SCRATCH/$side-tree" $flags -o "$out" >"$SCRATCH/$name.$side.log" 2>&1
    rc=$?
    [ "$rc" -eq 0 ] || { tail -5 "$SCRATCH/$name.$side.log" >&2; die "$name" "$side build exited $rc"; }
    [ -s "$out" ] || die "$name" "$side produced no output file"
  done
  cb="$(crc_size "$SCRATCH/$name.base.bin")"; cc="$(crc_size "$SCRATCH/$name.cand.bin")"
  [ -n "$cb" ] && [ -n "$cc" ] || die "$name" "could not CRC an output"
  if cmp -s "$SCRATCH/$name.base.bin" "$SCRATCH/$name.cand.bin"; then
    verdict=SAME; same=$((same+1))
  else
    verdict=DIFFER; differ=$((differ+1))
  fi
  echo "SHAPE $name $cb $cc $verdict"
  rm -f "$SCRATCH/$name.base.bin" "$SCRATCH/$name.cand.bin"
done

echo "SHAPES $same/$total IDENTICAL"
rc=0; [ "$same" -eq "$total" ] || rc=1
echo "STEP0_CMP_END rc=$rc"
exit $rc
