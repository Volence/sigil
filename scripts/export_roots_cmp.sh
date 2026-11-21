#!/usr/bin/env bash
# Export-text proof for G0b Task 13a (`G0B-EXPORT-BY-ROOTS`): a `files` row is (root name,
# path relative to that root's directory in this build), so an aeon game's `game` rows lose
# their leading `<game id>/` and NOTHING else in the export moves.
#
#   scripts/export_roots_cmp.sh --old <sigil> --new <sigil> --aeon-ref <dir>
#
# Builds the 7 shipped shapes (`sigil build --project <tree>/games/<game> --shape <row>`,
# export on) with the OLD binary (one built before the change, e.g. 02b8a8f4) and the NEW
# one, each in its own `cp -a` of the reference tree, and then, per shape, asserts:
#   * the old export with the leading `<shape.game>/` stripped from every `game`-root
#     `files[].path` (a game row WITHOUT that prefix is a failure: the old rule always
#     wrote it) equals the new export in every key EXCEPT `inputsSha256` and
#     `tool.revision`: the same ROM (`rom`), the same `files` in the same order (so every
#     file index is unchanged), the same `lines` rows, symbols, layouts, consts, defines;
#   * `inputsSha256` and `tool.revision` are reported (they are expected to move: the
#     revision is the binary's, and the inputs hash covers the tool and the root-relative
#     input paths).
# The two exports are read from the builds this script ran, never from a `.build.json`
# lying in a tree.
#
# Exit 0 = every shape SAME-MODULO-STRIP, 1 = at least one shape differs, 2 = could not run
# (a failed build, a missing export, an unreadable file: NEVER a pass). Output: one
#   EXPORT <shape> files=<n> game-rows=<k> line-rows=<m> rom=<sha256/16> inputsSha256=<moved|same> tool.revision=<moved|same> SAME-MODULO-STRIP|DIFFER
# row per shape (a DIFFER row is followed by the first differing keys), then
# `EXPORTS <k>/7 SAME-MODULO-STRIP`, then `EXPORT_ROOTS_CMP_END rc=<n>`.
#
# The reference tree must already hold aeon's generated debug inputs
# (`engine/debug/generated/`, written by aeon's build.sh), or the debug shapes fail to build
# (exit 2).
set -uo pipefail

OLD="" NEW="" REF=""
usage() { echo "usage: $0 --old <sigil> --new <sigil> --aeon-ref <dir>" >&2; }
while [ $# -gt 0 ]; do
  case "$1" in
    --old) OLD="${2:-}"; shift 2 ;;
    --new) NEW="${2:-}"; shift 2 ;;
    --aeon-ref) REF="${2:-}"; shift 2 ;;
    *) usage; echo "EXPORT_ROOTS_CMP_END rc=2"; exit 2 ;;
  esac
done

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRATCH="$HERE/.scratch-exportcmp-$$-$(date +%s)"
cleanup() { rm -rf "$SCRATCH"; }
trap cleanup EXIT
die() { echo "COULD NOT RUN ${1}: ${2}"; echo "EXPORT_ROOTS_CMP_END rc=2"; exit 2; }

[ -n "$OLD" ] || die setup "missing --old"
[ -n "$NEW" ] || die setup "missing --new"
[ -n "$REF" ] || die setup "missing --aeon-ref"
[ -x "$OLD" ] || die setup "old binary not executable: $OLD"
[ -x "$NEW" ] || die setup "new binary not executable: $NEW"
[ -d "$REF" ] || die setup "reference tree missing: $REF"
command -v python3 >/dev/null || die setup "python3 is not on PATH"

# name|game|[shapes.*] row: the same 7 shapes scripts/step0_shapes_cmp.sh compares.
SHAPES=(
  "plain|sonic4|release"
  "debug|sonic4|debug"
  "demo|demo|release"
  "demo_debug|demo|debug"
  "config_a|sonic4|config-a"
  "config_b|sonic4|config-b"
  "lean|sonic4|lean"
)

mkdir -p "$SCRATCH" || die setup "cannot create $SCRATCH"
for side in old new; do
  cp -a "$REF" "$SCRATCH/$side-tree" || die setup "copy of the reference tree failed"
  find "$SCRATCH/$side-tree" -name __pycache__ -type d -prune -exec rm -rf {} + 2>/dev/null
  [ -f "$SCRATCH/$side-tree/.git" ] && rm -f "$SCRATCH/$side-tree/.git"
  mkdir -p "$SCRATCH/$side-out"
done

echo "EXPORT_ROOTS_CMP old=$OLD ($("$OLD" --version 2>/dev/null | sed -n 's/^ *revision: *//p'))"
echo "EXPORT_ROOTS_CMP new=$NEW ($("$NEW" --version 2>/dev/null | sed -n 's/^ *revision: *//p'))"
echo "EXPORT_ROOTS_CMP ref=$REF"

same=0
total=${#SHAPES[@]}
for row in "${SHAPES[@]}"; do
  IFS='|' read -r name game shape <<<"$row"
  for side in old new; do
    bin="$OLD"; [ "$side" = new ] && bin="$NEW"
    out="$SCRATCH/$side-out/$name.bin"
    "$bin" build --project "$SCRATCH/$side-tree/games/$game" --shape "$shape" -o "$out" \
      >"$SCRATCH/$side-out/$name.log" 2>&1
    rc=$?
    [ "$rc" -eq 0 ] || { tail -n 5 "$SCRATCH/$side-out/$name.log" >&2; die "$name" "$side build exited $rc"; }
    [ -s "$SCRATCH/$side-out/$name.build.json" ] || die "$name" "$side build wrote no export"
  done
  python3 - "$name" "$SCRATCH/old-out/$name.build.json" "$SCRATCH/new-out/$name.build.json" <<'PY'
import json, sys
name, old_p, new_p = sys.argv[1:4]
try:
    old = json.load(open(old_p))
    new = json.load(open(new_p))
except Exception as e:  # unreadable is "could not run", never a verdict
    print(f"COULD NOT RUN {name}: {e}")
    sys.exit(2)

problems = []
game = old.get("shape", {}).get("game")
if not isinstance(game, str) or not game:
    print(f"COULD NOT RUN {name}: the old export names no shape.game")
    sys.exit(2)
prefix = game + "/"
game_rows = 0
for i, f in enumerate(old.get("files", [])):
    if f.get("root") == "game":
        game_rows += 1
        if not f["path"].startswith(prefix):
            problems.append(f"old files[{i}] game row {f['path']!r} lacks the `{prefix}` prefix")
        else:
            f["path"] = f["path"][len(prefix):]

moved = {}
for key in ("inputsSha256",):
    moved[key] = "moved" if old.get(key) != new.get(key) else "same"
    old.pop(key, None); new.pop(key, None)
ot, nt = old.get("tool", {}), new.get("tool", {})
moved["tool.revision"] = "moved" if ot.get("revision") != nt.get("revision") else "same"
ot.pop("revision", None); nt.pop("revision", None)

def diff(a, b, path, out):
    if len(out) >= 8:
        return
    if type(a) is not type(b):
        out.append(f"{path}: {type(a).__name__} vs {type(b).__name__}")
    elif isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                out.append(f"{path}.{k}: only in {'new' if k not in a else 'old'}")
            else:
                diff(a[k], b[k], f"{path}.{k}", out)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append(f"{path}: {len(a)} vs {len(b)} entries")
        for i, (x, y) in enumerate(zip(a, b)):
            diff(x, y, f"{path}[{i}]", out)
    elif a != b:
        out.append(f"{path}: {str(a)[:80]!r} vs {str(b)[:80]!r}")

diff(old, new, "$", problems)
line_rows = sum(len(s.get("rows", [])) for s in new.get("lines", {}).get("sections", {}).values())
verdict = "DIFFER" if problems else "SAME-MODULO-STRIP"
print(f"EXPORT {name} files={len(new.get('files', []))} game-rows={game_rows} line-rows={line_rows} "
      f"rom={new.get('rom', {}).get('sha256', '?')[:16]} inputsSha256={moved['inputsSha256']} "
      f"tool.revision={moved['tool.revision']} {verdict}")
for p in problems:
    print(f"  {p}")
sys.exit(1 if problems else 0)
PY
  prc=$?
  case "$prc" in
    0) same=$((same+1)) ;;
    1) ;;
    *) echo "EXPORT_ROOTS_CMP_END rc=2"; exit 2 ;;
  esac
done

echo "EXPORTS $same/$total SAME-MODULO-STRIP"
rc=0; [ "$same" -eq "$total" ] || rc=1
echo "EXPORT_ROOTS_CMP_END rc=$rc"
exit $rc
