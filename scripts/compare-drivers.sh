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
# run in the same directory, `$WORK/out`, with the same relative `-o`/`--emit-lst` names
# (one after the other, then moved aside), so the output path the listing's source digest
# records (resolved to an absolute path) is the same for both.
#
# Compared per shape: the ROM's SHA-256 (exact), the `.lst` bytes and the G0a build export
# (`<stem>.build.json`) with `tool` and `inputsSha256` removed (spec: G0b 6, item 2).
#
# THE DECLARED DELTA (proxy ruling on QUEUE G0B-COMPARE-IDENTITY-NAMES, option a). The two
# drivers differ by construction in two things, and only those may differ:
#   * the shape's NAME: the game-as-data driver names a shape by its `[shapes.*]` row (the
#     row is checked to exist in the copy's game.toml), the legacy one by its flag/label;
#   * the description files the game-as-data driver reads: `games/<g>/game.toml` and
#     `<engine.root>/engine.toml` are rows of its listing's read set, and so (G0b Task 8)
#     are the files its contract closure gate reads its frozen rows from, where the legacy
#     driver's live in Sigil's source: the `baseline` file engine.toml names, the one the
#     game's game.toml names, and, because the closure analyses every scanned module until
#     Task 9, each OTHER `games/*/game.toml` and the `baseline` file it names.
# The job transforms the LEGACY listing and export into the artifacts the new driver must
# write (DIGEST-SHAPE `target=` -> the row name; exactly those DIGEST-READ rows, each file
# named from the copy's own descriptions, crc and size from the copy's own files, inserted
# in digest order; DIGEST-AGGREGATE recomputed;
# export `shape.shape` -> the row name, everything else untouched) and compares those with
# the new artifacts BYTE FOR BYTE. Nothing is removed from the new side, so an extra read,
# a missing one, or a wrong name is a DIFFERS. A passing column prints
# `MATCH(declared-delta)` and a passing row `MATCH (declared delta)`, never a bare MATCH.
#
# THE CONTRACT GATE, THE CLOSURE REPORT AND THE CHECK (G0b Task 8 step 0). Both drivers run
# the closure gate before they build; each states a pass on stderr (`contract closure gate:
# passed (`), so a log without it is a gate that did not run. Per shape the job also runs
# `--report contracts` and `--check` with both drivers:
#   gate=AGREE(passed)  both logs carry the pass line (AGREE(refused): both refused, the
#                       red-first shape; DIFFERS(<legacy>/<project>) otherwise);
#   report=MATCH        the two closure censuses are identical below their two header lines
#                       (the label line names the shape each driver's way; the define line
#                       must hold the same set);
#   check=MATCH         both checks exit 0 and their `checked:` counts are identical.
# `SIGIL_CONTRACTS` is removed from every run, so the opt-out can never fake a pass.
#
# Output: one row per shape,
#   SHAPE <name> game=<g> shape=<s> rom=<v> lst=<v> export=<v> gate=<v> report=<v> check=<v>
#         sha=<legacy>/<project> <VERDICT>
# with VERDICT `MATCH (declared delta)`, DIFFERS, or DID NOT RUN (a build failed or an
# artifact is missing), then `SHAPES <n>/<total> MATCH` and `COMPARE_DRIVERS_END rc=<rc>`.
# rc 0 = every shape matches; 1 = a shape DIFFERS; 2 = a shape DID NOT RUN or the job could
# not run at all. DID NOT RUN is never a pass. A DIFFERS row is followed by its differences
# (the diff of the expected listing against the new one, the differing export fields).
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
  # (native.rs), so --config-a is the legacy spelling of this fixture; its pairing row is
  # its own name. Its ROM is config-a's on both drivers: no separate byte evidence.
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

gate_of() { # log -> passed | refused | skipped | absent
  if grep -q '^contract closure gate: passed (' "$1"; then echo passed
  elif grep -q '^error: contract closure gate FAILED' "$1"; then echo refused
  elif grep -q 'contract closure gate SKIPPED' "$1"; then echo skipped
  else echo absent; fi
}

compare() { # name game shape gate report check -> prints the row; exit 0 MATCH (declared delta), 1 DIFFERS
  python3 -I - "$WORK/res/$1" "$1" "$2" "$3" "$WORK/project" "$4" "$5" "$6" <<'PY'
import hashlib, json, sys, zlib, os, difflib, tomllib
res, name, game, shape, ptree, gate, report, check = sys.argv[1:9]
L = lambda f: open(os.path.join(res, "legacy", f), "rb").read()
P = lambda f: open(os.path.join(res, "project", f), "rb").read()
rom_l, rom_p = L(name + ".bin"), P(name + ".bin")
lst_l, lst_p = L(name + ".lst"), P(name + ".lst")
exp_l, exp_p = json.loads(L(name + ".build.json")), json.loads(P(name + ".build.json"))
sha_l, sha_p = hashlib.sha256(rom_l).hexdigest(), hashlib.sha256(rom_p).hexdigest()
for e in (exp_l, exp_p):
    e.pop("tool", None); e.pop("inputsSha256", None)
problems = []

# The pairing: the expected name is the `[shapes.*]` row of the game description in the
# copy the new driver built from. A row name the description does not declare is refused.
g = tomllib.load(open(os.path.join(ptree, "games", game, "game.toml"), "rb"))
if shape not in g.get("shapes", {}):
    problems.append(f"pairing: games/{game}/game.toml declares no [shapes.{shape}] row")
eroot = g["engine"]["root"]
e = tomllib.load(open(os.path.join(ptree, eroot, "engine.toml"), "rb"))
descs = [f"games/{game}/game.toml", f"{eroot}/engine.toml", f"{eroot}/{e['baseline']}", f"games/{game}/{g['baseline']}"]
for other in sorted(os.listdir(os.path.join(ptree, "games"))):
    op = os.path.join(ptree, "games", other, "game.toml")
    if other != game and os.path.isfile(op):
        descs += [f"games/{other}/game.toml", f"games/{other}/{tomllib.load(open(op, 'rb'))['baseline']}"]

# THE DECLARED DELTA (proxy ruling, G0B-COMPARE-IDENTITY-NAMES option a). The OLD artifacts
# are transformed into the artifacts the new driver must write, and those are compared to
# the new ones byte for byte. Nothing is removed from the new side.
def expected_listing(old: bytes) -> bytes:
    lines = old.split(b"\n")
    shp = [i for i, x in enumerate(lines) if x.startswith(b"DIGEST-SHAPE ")]
    if len(shp) != 1:
        raise ValueError(f"legacy listing has {len(shp)} DIGEST-SHAPE line(s)")
    toks = lines[shp[0]].split(b" ")
    tgt = [k for k, t in enumerate(toks) if t.startswith(b"target=")]
    if len(tgt) != 1:
        raise ValueError("legacy DIGEST-SHAPE has no single target= field")
    toks[tgt[0]] = b"target=" + shape.encode()
    lines[shp[0]] = b" ".join(toks)
    reads = [i for i, x in enumerate(lines) if x.startswith(b"DIGEST-READ ")]
    if not reads or reads != list(range(reads[0], reads[-1] + 1)):
        raise ValueError("legacy listing's DIGEST-READ rows are not one block")
    rows = [lines[i] for i in reads]
    def key(row):  # (root, path): the --aeon root (no root= field) first, then by root name; path by bytes
        fixed, path = row.split(b" path=", 1)
        root = next((t[5:] for t in fixed.split(b" ") if t.startswith(b"root=")), None)
        return (0, b"", path) if root is None else (1, root, path)
    for rel in descs:
        if any(r.split(b" path=", 1)[1] == rel.encode() and b" root=" not in r.split(b" path=", 1)[0] for r in rows):
            raise ValueError(f"legacy listing already reads {rel}")
        data = open(os.path.join(ptree, rel), "rb").read()
        rows.append(f"DIGEST-READ crc={zlib.crc32(data) & 0xffffffff:08x} size={len(data)} origin=source path={rel}".encode())
    rows.sort(key=key)
    agg = [i for i, x in enumerate(lines) if x.startswith(b"DIGEST-AGGREGATE ")]
    if agg != [reads[-1] + 1]:
        raise ValueError("legacy listing's DIGEST-AGGREGATE does not follow its READ block")
    crc = zlib.crc32(b"".join(r + b"\n" for r in rows)) & 0xffffffff
    aggregate = f"DIGEST-AGGREGATE crc={crc:08x} reads={len(rows)}".encode()
    return b"\n".join(lines[:reads[0]] + rows + [aggregate] + lines[agg[0] + 1:])

def expected_export(old: dict) -> dict:
    e = json.loads(json.dumps(old))
    if not isinstance(e.get("shape"), dict) or "shape" not in e["shape"]:
        raise ValueError("legacy export has no shape.shape")
    e["shape"]["shape"] = shape
    return e

rom_ok = sha_l == sha_p
try:
    want_lst = expected_listing(lst_l)
except ValueError as err:
    want_lst = None; problems.append(f"lst: cannot apply the declared delta: {err}")
try:
    want_exp = expected_export(exp_l)
except ValueError as err:
    want_exp = None; problems.append(f"export: cannot apply the declared delta: {err}")
lst_ok = want_lst is not None and want_lst == lst_p
exp_ok = want_exp is not None and want_exp == exp_p
ok = rom_ok and lst_ok and exp_ok and gate == "AGREE(passed)" and report == "MATCH" and check == "MATCH" and not problems
D = "MATCH(declared-delta)"
col = lambda good, tag: tag if good else "DIFFERS"
print(f"SHAPE {name} game={game} shape={shape} rom={col(rom_ok, 'MATCH')} lst={col(lst_ok, D)} export={col(exp_ok, D)} "
      f"gate={gate} report={report} check={check} sha={sha_l[:16]}/{sha_p[:16]} {'MATCH (declared delta)' if ok else 'DIFFERS'}")
for pr in problems:
    print("    " + pr)
if ok:
    sys.exit(0)
if not rom_ok:
    n = min(len(rom_l), len(rom_p))
    first = next((i for i in range(n) if rom_l[i] != rom_p[i]), n)
    diff = sum(1 for i in range(n) if rom_l[i] != rom_p[i]) + abs(len(rom_l) - len(rom_p))
    print(f"    rom: legacy {len(rom_l)} B, project {len(rom_p)} B, {diff} byte(s) differ, first at {first:#x}")
if want_lst is not None and not lst_ok:
    d = [x for x in difflib.unified_diff(want_lst.decode(errors="replace").splitlines(), lst_p.decode(errors="replace").splitlines(),
                                         "expected.lst (legacy + declared delta)", "project.lst", n=0, lineterm="") if not x.startswith("@@")]
    for x in d[:24]:
        print("    lst: " + x)
    if len(d) > 24:
        print(f"    lst: ... {len(d) - 24} more diff line(s)")
if want_exp is not None and not exp_ok:
    for k in sorted(set(want_exp) | set(exp_p)):
        if want_exp.get(k) != exp_p.get(k):
            print(f"    export.{k}: expected {json.dumps(want_exp.get(k))[:160]}")
            print(f"    export.{k}: project  {json.dumps(exp_p.get(k))[:160]}")
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
  ( cd "$WORK/out" && env -u AEON_DIR -u SIGIL_CONTRACTS "$SIGIL" build --legacy --aeon "$WORK/legacy" $flags \
      -o "$name.bin" --emit-lst "$name.lst" ) > "$WORK/logs/$name.legacy.log" 2>&1
  lrc=$?
  mv "$WORK/out/$name".* "$WORK/res/$name/legacy/" 2>/dev/null
  ( cd "$WORK/out" && env -u AEON_DIR -u SIGIL_CONTRACTS "$SIGIL" build --project "$WORK/project/games/$game" --shape "$shape" \
      -o "$name.bin" --emit-lst "$name.lst" ) > "$WORK/logs/$name.project.log" 2>&1
  prc=$?
  mv "$WORK/out/$name".* "$WORK/res/$name/project/" 2>/dev/null
  gl=$(gate_of "$WORK/logs/$name.legacy.log") gp=$(gate_of "$WORK/logs/$name.project.log")
  if [ "$gl" = "$gp" ] && { [ "$gl" = passed ] || [ "$gl" = refused ]; }; then gate="AGREE($gl)"; else gate="DIFFERS($gl/$gp)"; fi
  # The closure census and the check, each driver its own way.
  # shellcheck disable=SC2086
  env -u AEON_DIR -u SIGIL_CONTRACTS "$SIGIL" build --legacy --aeon "$WORK/legacy" $flags --report contracts \
      > "$WORK/res/$name/legacy/report.txt" 2> "$WORK/logs/$name.legacy.report.log"
  lrr=$?
  env -u AEON_DIR -u SIGIL_CONTRACTS "$SIGIL" build --project "$WORK/project/games/$game" --shape "$shape" --report contracts \
      > "$WORK/res/$name/project/report.txt" 2> "$WORK/logs/$name.project.report.log"
  prr=$?
  report=DIFFERS
  if [ $lrr = 0 ] && [ $prr = 0 ] && [ -s "$WORK/res/$name/legacy/report.txt" ] \
     && cmp -s <(tail -n +3 "$WORK/res/$name/legacy/report.txt") <(tail -n +3 "$WORK/res/$name/project/report.txt") \
     && [ "$(sed -n 2p "$WORK/res/$name/legacy/report.txt" | tr ' ' '\n' | sort)" = "$(sed -n 2p "$WORK/res/$name/project/report.txt" | tr ' ' '\n' | sort)" ]; then
    report=MATCH
  elif [ $lrr != 0 ] || [ $prr != 0 ]; then report="DID-NOT-RUN($lrr/$prr)"; fi
  # shellcheck disable=SC2086
  env -u AEON_DIR -u SIGIL_CONTRACTS "$SIGIL" build --legacy --aeon "$WORK/legacy" $flags --check \
      > "$WORK/res/$name/legacy/check.txt" 2> "$WORK/logs/$name.legacy.check.log"
  lcr=$?
  env -u AEON_DIR -u SIGIL_CONTRACTS "$SIGIL" build --project "$WORK/project/games/$game" --shape "$shape" --check \
      > "$WORK/res/$name/project/check.txt" 2> "$WORK/logs/$name.project.check.log"
  pcr=$?
  counts() { grep '^checked: ' "$1" | sed 's/^checked: [^:]*: //'; }
  check=DIFFERS
  if [ $lcr != 0 ] || [ $pcr != 0 ]; then check="DID-NOT-RUN($lcr/$pcr)"
  elif [ -n "$(counts "$WORK/res/$name/legacy/check.txt")" ] \
     && [ "$(counts "$WORK/res/$name/legacy/check.txt")" = "$(counts "$WORK/res/$name/project/check.txt")" ]; then check=MATCH; fi
  missing=""
  for side in legacy project; do
    for ext in bin lst build.json; do
      [ -s "$WORK/res/$name/$side/$name.$ext" ] || missing="$missing $side/$name.$ext"
    done
  done
  if [ $lrc != 0 ] || [ $prc != 0 ] || [ -n "$missing" ]; then
    echo "SHAPE $name game=$game shape=$shape legacy-rc=$lrc project-rc=$prc gate=$gate report=$report check=$check missing:${missing:- none} DID NOT RUN"
    for side in legacy project; do
      tail -n 6 "$WORK/logs/$name.$side.log" | sed "s/^/    $side: /"
    done
    dnr=$((dnr + 1))
    continue
  fi
  if compare "$name" "$game" "$shape" "$gate" "$report" "$check"; then match=$((match + 1)); else differ=$((differ + 1)); fi
done

[ "$total" -gt 0 ] || die "--only '$ONLY' selects no shape"
echo "SHAPES $match/$total MATCH"
rc=0
[ $differ = 0 ] || rc=1
[ $dnr = 0 ] || rc=2
echo "COMPARE_DRIVERS_END rc=$rc"
exit $rc
