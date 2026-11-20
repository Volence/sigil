#!/usr/bin/env bash
# The G0b comparison job (Task 7): build every shape with BOTH of Sigil's build drivers
# from one aeon tree and require the same artifacts.
#
#   scripts/compare-drivers.sh --aeon <aeon git tree> [--sigil <sigil>] [--work <dir>]
#                              [--only <name,name,...>] [--keep]
#
#   * the profile-driven driver:  sigil build --legacy --aeon <copy L> <flags>
#   * the game-as-data driver:    sigil build --project <copy P>/games/<g> --shape <s>
#
# Each driver builds from its OWN run-unique copy of the tree (a `git clone` checked out
# at the tree's HEAD, with the tree's uncommitted tracked changes applied on top, so a
# mutation planted in the tree reaches both copies). A worktree is never `cp -a`'d: its
# `.git` is a file that would share the original's HEAD and index. Both builds of a shape
# write to the SAME absolute path (one after the other, then moved aside), because the
# listing's source digest names the output path.
#
# Compared per shape: the ROM's SHA-256, the `.lst` bytes, and the G0a build export
# (`<stem>.build.json`) with `tool` and `inputsSha256` removed (spec: G0b 6, item 2).
# Nothing else is removed. Output: one row per shape,
#   SHAPE <name> game=<g> shape=<s> rom=<v> lst=<v> export=<v> sha=<legacy>/<project> <VERDICT>
# with VERDICT MATCH, DIFFERS, or DID NOT RUN (a build failed or an artifact is missing),
# then `SHAPES <n>/<total> MATCH` and `COMPARE_DRIVERS_END rc=<rc>`.
# rc 0 = every shape MATCH; 1 = a shape DIFFERS; 2 = a shape DID NOT RUN or the job could
# not run at all. DID NOT RUN is never a pass.
#
# A DIFFERS row is followed by its differences (the `.lst` diff, the differing export
# fields), and by one `declared-delta` line saying whether the difference is exactly the
# two the drivers differ in by construction today: the shape's NAME (the legacy driver
# names it by its flag/profile label, the game-as-data driver by its `[shapes.*]` row) and
# the description files the game-as-data driver reads (`game.toml`, `engine.toml`), which
# are rows of its listing's read set. The line is diagnostic only: it never turns a
# DIFFERS into a MATCH (QUEUE G0B-COMPARE-IDENTITY-NAMES holds the open ruling).
#
# The shape table below pairs each `[shapes.*]` row of aeon's game descriptions with the
# legacy flags that select the same profile. `crates/sigil-cli/tests/driver_comparison.rs`
# derives the expected rows from the descriptions and fails on a row this table lacks.
set -uo pipefail

SIGIL="${SIGIL_BUILD:-}" AEON="" WORK="" ONLY="" KEEP=0
usage() { echo "usage: $0 --aeon <aeon git tree> [--sigil <sigil>] [--work <dir>] [--only a,b] [--keep]" >&2; }
while [ $# -gt 0 ]; do
  case "$1" in
    --aeon) AEON="${2:-}"; shift 2 ;;
    --sigil) SIGIL="${2:-}"; shift 2 ;;
    --work) WORK="${2:-}"; shift 2 ;;
    --only) ONLY="${2:-}"; shift 2 ;;
    --keep) KEEP=1; shift ;;
    *) usage; echo "COMPARE_DRIVERS_END rc=2"; exit 2 ;;
  esac
done

die() { echo "COULD NOT RUN: $*"; echo "COMPARE_DRIVERS_END rc=2"; exit 2; }

# name|game|shape|legacy flags
ROWS=(
  "release|sonic4|release|"
  "debug|sonic4|debug|--debug"
  "demo|demo|release|--game demo"
  "demo_debug|demo|debug|--game demo --debug"
  "config_a|sonic4|config-a|--config-a"
  "config_b|sonic4|config-b|--config-b"
  "lean|sonic4|lean|--lean"
  # The legacy driver's config_a_keystones_flipped_profile() IS config_a_profile()
  # (native.rs), so --config-a is the legacy spelling of this fixture.
  "config_a_keystones_flipped|sonic4|config-a-keystones-flipped|--config-a"
  "stress_evict|sonic4|stress-evict|--stress-evict"
  "stress_art|sonic4|stress-art|--stress-art"
)

[ -n "$SIGIL" ] || die "no sigil binary (pass --sigil or set SIGIL_BUILD)"
[ -x "$SIGIL" ] || die "sigil binary not executable: $SIGIL"
SIGIL="$(cd "$(dirname "$SIGIL")" && pwd)/$(basename "$SIGIL")"
[ -n "$AEON" ] || die "missing --aeon"
[ -d "$AEON" ] || die "aeon tree missing: $AEON"
AEON="$(cd "$AEON" && pwd)"
[ -f "$AEON/build.sh" ] || die "$AEON has no build.sh; not an aeon tree"
git -C "$AEON" rev-parse --is-inside-work-tree >/dev/null 2>&1 \
  || die "$AEON is not a git work tree, so no clean copy of it can be made"
HEAD_SHA="$(git -C "$AEON" rev-parse HEAD)" || die "cannot read HEAD of $AEON"
# An untracked file would be in the tree but in neither copy: the copies would not be it.
UNTRACKED="$(git -C "$AEON" status --porcelain --untracked-files=normal | grep '^??' || true)"
[ -z "$UNTRACKED" ] || die "$AEON has untracked files the copies would not carry: $(echo "$UNTRACKED" | head -5 | tr '\n' ' ')"
DIRTY="$(git -C "$AEON" status --porcelain --untracked-files=no | wc -l)"

if [ -z "$WORK" ]; then
  WORK="$(dirname "$AEON")/.compare-drivers-$(basename "$AEON")-$$-$(date +%s)"
fi
[ -e "$WORK" ] && die "work directory already exists: $WORK"
mkdir -p "$WORK" || die "cannot create $WORK"
WORK="$(cd "$WORK" && pwd)"
cleanup() { [ "$KEEP" = 1 ] || rm -rf "$WORK"; }
trap cleanup EXIT

echo "COMPARE sigil=$SIGIL ($("$SIGIL" --version 2>/dev/null | head -1))"
echo "COMPARE aeon=$AEON head=$HEAD_SHA uncommitted-tracked=$DIRTY work=$WORK"

copy_tree() { # side
  local dst="$WORK/$1"
  git clone -q --no-checkout "$AEON" "$dst" || return 1
  git -C "$dst" checkout -q --detach "$HEAD_SHA" || return 1
  if [ "$DIRTY" != 0 ]; then
    git -C "$AEON" diff --binary HEAD > "$WORK/$1.patch" || return 1
    git -C "$dst" apply --whitespace=nowarn "$WORK/$1.patch" || return 1
  fi
  find "$dst" -name __pycache__ -type d -prune -exec rm -rf {} + || return 1
  # The one build-time-generated input a sigil build reads that git does not carry: the
  # compression self-test vectors (engine/debug/generated), produced as build.sh does.
  ( cd "$dst" \
    && { [ -x tools/bin/salvador ] || { make -C tools/salvador -s && mkdir -p tools/bin && cp tools/salvador/salvador tools/bin/salvador; }; } \
    && python3 tools/gen_compression_vectors.py ) > "$WORK/$1.provision.log" 2>&1 || return 1
}
for side in legacy project; do
  copy_tree "$side" || { tail -n 20 "$WORK/$side.provision.log" 2>/dev/null; die "copy of $AEON for the $side driver failed"; }
done
mkdir -p "$WORK/out" "$WORK/res" "$WORK/logs"

compare() { # name game shape -> prints the row; exit 0 MATCH, 1 DIFFERS
  python3 -I - "$WORK/res/$1" "$1" "$2" "$3" "$WORK/project" <<'PY'
import hashlib, json, sys, zlib, os, difflib
res, name, game, shape, ptree = sys.argv[1:6]
L = lambda f: open(os.path.join(res, "legacy", f), "rb").read()
P = lambda f: open(os.path.join(res, "project", f), "rb").read()
rom_l, rom_p = L(name + ".bin"), P(name + ".bin")
lst_l, lst_p = L(name + ".lst"), P(name + ".lst")
exp_l, exp_p = json.loads(L(name + ".build.json")), json.loads(P(name + ".build.json"))
sha_l, sha_p = hashlib.sha256(rom_l).hexdigest(), hashlib.sha256(rom_p).hexdigest()
for e in (exp_l, exp_p):
    e.pop("tool", None); e.pop("inputsSha256", None)
v = lambda same: "MATCH" if same else "DIFFERS"
rom_ok, lst_ok, exp_ok = sha_l == sha_p, lst_l == lst_p, exp_l == exp_p
verdict = v(rom_ok and lst_ok and exp_ok)
print(f"SHAPE {name} game={game} shape={shape} rom={v(rom_ok)} lst={v(lst_ok)} export={v(exp_ok)} "
      f"sha={sha_l[:16]}/{sha_p[:16]} {verdict}")
if verdict == "MATCH":
    sys.exit(0)
if not rom_ok:
    n = min(len(rom_l), len(rom_p))
    first = next((i for i in range(n) if rom_l[i] != rom_p[i]), n)
    diff = sum(1 for i in range(n) if rom_l[i] != rom_p[i]) + abs(len(rom_l) - len(rom_p))
    print(f"    rom: legacy {len(rom_l)} B, project {len(rom_p)} B, {diff} byte(s) differ, first at {first:#x}")
ll, pl = lst_l.decode(errors="replace").splitlines(), lst_p.decode(errors="replace").splitlines()
if not lst_ok:
    d = [x for x in difflib.unified_diff(ll, pl, "legacy.lst", "project.lst", n=0, lineterm="") if not x.startswith("@@")]
    for x in d[:24]:
        print("    lst: " + x)
    if len(d) > 24:
        print(f"    lst: ... {len(d) - 24} more diff line(s)")
keys = sorted(set(exp_l) | set(exp_p))
bad = [k for k in keys if exp_l.get(k) != exp_p.get(k)]
for k in bad:
    a, b = json.dumps(exp_l.get(k))[:160], json.dumps(exp_p.get(k))[:160]
    print(f"    export.{k}: legacy {a}")
    print(f"    export.{k}: project {b}")
# Diagnostic: is the difference exactly the declared one (shape name + description reads)?
decl = []
try:
    import tomllib
    g = tomllib.load(open(os.path.join(ptree, "games", game, "game.toml"), "rb"))
    descs = [f"games/{game}/game.toml", f"{g['engine']['root']}/engine.toml"]
    def row(rel):
        d = open(os.path.join(ptree, rel), "rb").read()
        return f"DIGEST-READ crc={zlib.crc32(d) & 0xffffffff:08x} size={len(d)} origin=source path={rel}"
    want_rows = {row(r) for r in descs}
    lo = [x for x in ll if not x.startswith(("DIGEST-SHAPE ", "DIGEST-AGGREGATE "))]
    po = [x for x in pl if not x.startswith(("DIGEST-SHAPE ", "DIGEST-AGGREGATE ")) and x not in want_rows]
    shp_l = [x for x in ll if x.startswith("DIGEST-SHAPE ")]
    shp_p = [x for x in pl if x.startswith("DIGEST-SHAPE ")]
    def strip_target(x):
        return " ".join(t for t in x.split() if not t.startswith("target="))
    reads_p = [x for x in pl if x.startswith("DIGEST-READ ")]
    agg_p = [x for x in pl if x.startswith("DIGEST-AGGREGATE ")]
    agg_ok = len(agg_p) == 1 and agg_p[0] == f"DIGEST-AGGREGATE crc={zlib.crc32(''.join(r + chr(10) for r in reads_p).encode()) & 0xffffffff:08x} reads={len(reads_p)}"
    lst_decl = (lo == po and len(shp_l) == len(shp_p) == 1 and strip_target(shp_l[0]) == strip_target(shp_p[0])
                and f"target={shape}" in shp_p[0].split() and want_rows <= set(pl) and agg_ok)
    el, ep = dict(exp_l), dict(exp_p)
    el["shape"] = dict(el.get("shape", {})); el["shape"]["shape"] = shape
    exp_decl = el == ep
    print(f"    declared-delta: lst={'yes' if lst_decl else 'NO'} export={'yes' if exp_decl else 'NO'} rom={'n/a' if rom_ok else 'NO'} "
          f"(shape name + reads of {', '.join(descs)}; diagnostic only, the verdict stays DIFFERS)")
except Exception as e:  # never a verdict, so a failure to classify is only printed
    print(f"    declared-delta: could not classify: {e}")
sys.exit(1)
PY
}

total=0 match=0 differ=0 dnr=0
for row in "${ROWS[@]}"; do
  IFS='|' read -r name game shape flags <<< "$row"
  if [ -n "$ONLY" ] && ! printf '%s\n' "${ONLY//,/$'\n'}" | grep -qx "$name"; then continue; fi
  total=$((total + 1))
  mkdir -p "$WORK/res/$name/legacy" "$WORK/res/$name/project"
  # shellcheck disable=SC2086
  ( cd "$WORK/out" && env -u AEON_DIR "$SIGIL" build --legacy --aeon "$WORK/legacy" $flags \
      -o "$name.bin" --emit-lst "$name.lst" ) > "$WORK/logs/$name.legacy.log" 2>&1
  lrc=$?
  mv "$WORK/out/$name".* "$WORK/res/$name/legacy/" 2>/dev/null
  ( cd "$WORK/out" && env -u AEON_DIR "$SIGIL" build --project "$WORK/project/games/$game" --shape "$shape" \
      -o "$name.bin" --emit-lst "$name.lst" ) > "$WORK/logs/$name.project.log" 2>&1
  prc=$?
  mv "$WORK/out/$name".* "$WORK/res/$name/project/" 2>/dev/null
  missing=""
  for side in legacy project; do
    for ext in bin lst build.json; do
      [ -s "$WORK/res/$name/$side/$name.$ext" ] || missing="$missing $side/$name.$ext"
    done
  done
  if [ $lrc != 0 ] || [ $prc != 0 ] || [ -n "$missing" ]; then
    echo "SHAPE $name game=$game shape=$shape legacy-rc=$lrc project-rc=$prc missing:${missing:- none} DID NOT RUN"
    for side in legacy project; do
      tail -n 6 "$WORK/logs/$name.$side.log" | sed "s/^/    $side: /"
    done
    dnr=$((dnr + 1))
    continue
  fi
  if compare "$name" "$game" "$shape"; then match=$((match + 1)); else differ=$((differ + 1)); fi
done

[ "$total" -gt 0 ] || die "--only '$ONLY' selects no shape"
echo "SHAPES $match/$total MATCH"
rc=0
[ $differ = 0 ] || rc=1
[ $dnr = 0 ] || rc=2
echo "COMPARE_DRIVERS_END rc=$rc"
exit $rc
