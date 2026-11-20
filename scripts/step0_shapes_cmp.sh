#!/usr/bin/env bash
# Byte-neutral proof for the step-0 soundness plan: build the 7 shipped shapes with a
# BASE and a CANDIDATE sigil and compare the ROM bytes.
#
#   scripts/step0_shapes_cmp.sh --base <sigil> --cand <sigil> --aeon-ref <dir>
#   scripts/step0_shapes_cmp.sh --base <sigil> --base-ref <dir> [--base-grammar aeon|project]
#                               --cand <sigil> --cand-ref <dir> [--cand-grammar aeon|project]
#
# Each side is invoked in its OWN build grammar against its OWN reference tree (G0b Task 11:
# the candidate no longer speaks the profile-driven grammar, while the base binary,
# .g0b-base-walk-87b5c195, speaks nothing else):
#   aeon     sigil build --aeon <tree> [--game demo] [--debug] [--config-a|--config-b|--lean]
#            (the profile-driven driver, deleted by G0b Task 11; old binaries only)
#   project  sigil build --project <tree>/games/<game> --shape <row>
#            (the game-as-data driver; the tree must carry the G0b descriptions)
# `--aeon-ref <dir>` sets both trees; each side's grammar defaults to `aeon` (the pre-Task-11
# spelling of this command, which a current candidate refuses: a build failure is exit 2).
# A side's tree is that side's whole input: the two trees may differ (the base's reference
# tree predates the descriptions), and the comparison is still bytes against bytes.
#
# Exit 0 = all 7 identical, 1 = at least one differs, 2 = could not run (a failed
# build or missing file is NEVER a pass). Output: one
#   SHAPE <name> <crc32>/<size> <crc32>/<size> SAME|DIFFER
# row per shape, then `SHAPES <k>/7 IDENTICAL`, then `STEP0_CMP_END rc=<n>`.
#
# A `sigil build` from before G0b Task 6 rewrites files inside the tree it builds (Sigil
# audit finding 9), and a BASE binary may be one, so each side gets its own `cp -a` of its
# reference tree under a run-unique scratch dir. The scratch goes ON DISK in the worktree
# (.scratch-cmp-*), not /tmp (RAM), and is removed at the end of every run. A copy of a git
# worktree drops its `.git` link file, so nothing done in the copy can reach the original.
#
# The shape table pairs each shape's profile flags (`sigil build --help` before Task 11:
#   [--game sonic4|demo] [--debug] [--config-a|--config-b|--lean])
# with the game-as-data `[shapes.*]` row aeon's descriptions give the same shape. A project
# side whose tree lacks a row is refused before anything is built.
set -uo pipefail

BASE="" CAND="" REF="" BREF="" CREF="" BGRAM="aeon" CGRAM="aeon"
usage() {
  echo "usage: $0 --base <sigil> --cand <sigil> (--aeon-ref <dir> | --base-ref <dir> --cand-ref <dir>)" >&2
  echo "          [--base-grammar aeon|project] [--cand-grammar aeon|project]" >&2
}
while [ $# -gt 0 ]; do
  case "$1" in
    --base) BASE="${2:-}"; shift 2 ;;
    --cand) CAND="${2:-}"; shift 2 ;;
    --aeon-ref) REF="${2:-}"; shift 2 ;;
    --base-ref) BREF="${2:-}"; shift 2 ;;
    --cand-ref) CREF="${2:-}"; shift 2 ;;
    --base-grammar) BGRAM="${2:-}"; shift 2 ;;
    --cand-grammar) CGRAM="${2:-}"; shift 2 ;;
    *) usage; echo "STEP0_CMP_END rc=2"; exit 2 ;;
  esac
done

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRATCH="$HERE/.scratch-cmp-$$-$(date +%s)"
cleanup() { rm -rf "$SCRATCH"; }
trap cleanup EXIT

die() { echo "COULD NOT RUN ${1}: ${2}"; echo "STEP0_CMP_END rc=2"; exit 2; }

[ -n "$BREF" ] || BREF="$REF"
[ -n "$CREF" ] || CREF="$REF"
[ -n "$BASE" ] || die setup "missing --base"
[ -n "$CAND" ] || die setup "missing --cand"
[ -n "$BREF" ] || die setup "missing --base-ref (or --aeon-ref)"
[ -n "$CREF" ] || die setup "missing --cand-ref (or --aeon-ref)"
for g in "$BGRAM" "$CGRAM"; do
  case "$g" in aeon|project) ;; *) die setup "unknown grammar '$g' (want aeon or project)" ;; esac
done
[ -x "$BASE" ] || die setup "base binary not executable: $BASE"
[ -x "$CAND" ] || die setup "cand binary not executable: $CAND"
[ -d "$BREF" ] || die setup "base reference tree missing: $BREF"
[ -d "$CREF" ] || die setup "cand reference tree missing: $CREF"
BASE="$(cd "$(dirname "$BASE")" && pwd)/$(basename "$BASE")"
CAND="$(cd "$(dirname "$CAND")" && pwd)/$(basename "$CAND")"

# name|profile flags|game|[shapes.*] row
SHAPES=(
  "plain||sonic4|release"
  "debug|--debug|sonic4|debug"
  "demo|--game demo|demo|release"
  "demo_debug|--game demo --debug|demo|debug"
  "config_a|--config-a|sonic4|config-a"
  "config_b|--config-b|sonic4|config-b"
  "lean|--lean|sonic4|lean"
)

# A project side reads its shape from the tree's own description: refuse a missing row
# before any build, so a renamed row is a setup failure, not a mismatch.
for side in base cand; do
  gram="$BGRAM"; tree="$BREF"; [ "$side" = cand ] && { gram="$CGRAM"; tree="$CREF"; }
  [ "$gram" = project ] || continue
  for row in "${SHAPES[@]}"; do
    IFS='|' read -r _name _flags game shape <<<"$row"
    desc="$tree/games/$game/game.toml"
    [ -f "$desc" ] || die setup "$side tree has no $desc (a project side needs the G0b descriptions)"
    grep -qE "^\[shapes\.(\"$shape\"|$shape)\]" "$desc" || die setup "$side tree's $desc has no [shapes.$shape] row"
  done
done

mkdir -p "$SCRATCH" || die setup "cannot create $SCRATCH"
cp -a "$BREF" "$SCRATCH/base-tree" || die setup "copy of the base reference tree failed"
cp -a "$CREF" "$SCRATCH/cand-tree" || die setup "copy of the cand reference tree failed"
find "$SCRATCH/base-tree" "$SCRATCH/cand-tree" -name __pycache__ -type d -prune -exec rm -rf {} + 2>/dev/null
# A worktree's `.git` is a FILE naming the main repo's gitdir: a copy carrying it would share
# the original's HEAD and index. No build reads the tree's git, so the copy drops the link.
for t in "$SCRATCH/base-tree" "$SCRATCH/cand-tree"; do [ -f "$t/.git" ] && rm -f "$t/.git"; done

echo "STEP0_CMP base=$BASE grammar=$BGRAM ref=$BREF"
echo "STEP0_CMP cand=$CAND grammar=$CGRAM ref=$CREF"

crc_size() { # file -> "<crc32 hex8>/<size>"; empty on failure
  python3 - "$1" <<'PY' 2>/dev/null
import sys, zlib
d = open(sys.argv[1], 'rb').read()
print("%08x/%d" % (zlib.crc32(d) & 0xffffffff, len(d)))
PY
}

same=0
total=${#SHAPES[@]}
for row in "${SHAPES[@]}"; do
  IFS='|' read -r name flags game shape <<<"$row"
  for side in base cand; do
    bin="$BASE"; gram="$BGRAM"
    [ "$side" = cand ] && { bin="$CAND"; gram="$CGRAM"; }
    tree="$SCRATCH/$side-tree"
    out="$SCRATCH/$name.$side.bin"
    if [ "$gram" = aeon ]; then
      # shellcheck disable=SC2086
      "$bin" build --aeon "$tree" $flags -o "$out" >"$SCRATCH/$name.$side.log" 2>&1
    else
      "$bin" build --project "$tree/games/$game" --shape "$shape" -o "$out" >"$SCRATCH/$name.$side.log" 2>&1
    fi
    rc=$?
    [ "$rc" -eq 0 ] || { tail -5 "$SCRATCH/$name.$side.log" >&2; die "$name" "$side build ($gram grammar) exited $rc"; }
    [ -s "$out" ] || die "$name" "$side produced no output file"
  done
  cb="$(crc_size "$SCRATCH/$name.base.bin")"; cc="$(crc_size "$SCRATCH/$name.cand.bin")"
  [ -n "$cb" ] && [ -n "$cc" ] || die "$name" "could not CRC an output"
  if cmp -s "$SCRATCH/$name.base.bin" "$SCRATCH/$name.cand.bin"; then
    verdict=SAME; same=$((same+1))
  else
    verdict=DIFFER
  fi
  echo "SHAPE $name $cb $cc $verdict"
  rm -f "$SCRATCH/$name.base.bin" "$SCRATCH/$name.cand.bin"
done

echo "SHAPES $same/$total IDENTICAL"
rc=0; [ "$same" -eq "$total" ] || rc=1
echo "STEP0_CMP_END rc=$rc"
exit $rc
