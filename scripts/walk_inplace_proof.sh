#!/usr/bin/env bash
# The byte-moving proof for parcel/walk-measure-in-place: build every shape with the OLD
# sigil (master) and the NEW one (this branch) from the same aeon reference, then run
# scripts/walk_inplace_proof.py on each pair. Every changed byte must be explained.
#
#   scripts/walk_inplace_proof.sh --old <sigil> --new <sigil> --aeon-ref <dir> --out <dir> \
#       [--fr1 <dir with exp_<shape>.bin>] [--keep]
#
# Exit 0 = every shape EXPLAINED, 1 = a shape has an unexplained byte or failed check,
# 2 = could not run (a failed build, a missing file or a could-not-measure is NEVER a pass).
# Output: one `PROOF <shape> ...` verdict per shape, `WALK_PROOF <k>/<n> EXPLAINED`, then
# `WALK_PROOF_END rc=<n>`. Per-shape detail goes to <out>/<shape>.proof.txt and
# <out>/<shape>.json; with --keep the ROM/lst/build.json triples stay in <out>/roms/.
#
# THE SHAPES. The seven shipped shapes and the two stress fixtures are the `sigil build`
# selectors (`sigil build --help`). The third fixture the parcel names,
# `config_a_keystones_flipped`, is `config_a_profile()` itself
# (`sigil-harness/src/native.rs`, `config_a_keystones_flipped_profile`), so its row IS
# config_a's; the runner checks that alias in the source rather than assuming it.
#
# THE FROZEN TABLE per shape is the one the OLD walk read (`frozen_sizes` in each profile
# of native.rs): it decides which sections the old walk measured in place. The stress
# fixtures are the sonic4 debug profile, so they read s4_debug.txt.
#
# `sigil build` rewrites generated files inside the tree it builds, so each binary gets
# its own `cp -a` of the reference tree, under a run-unique scratch dir in this worktree,
# removed at the end.
set -uo pipefail

OLD="" NEW="" REF="" OUT="" FR1="" KEEP=0
while [ $# -gt 0 ]; do
  case "$1" in
    --old) OLD="${2:-}"; shift 2 ;;
    --new) NEW="${2:-}"; shift 2 ;;
    --aeon-ref) REF="${2:-}"; shift 2 ;;
    --out) OUT="${2:-}"; shift 2 ;;
    --fr1) FR1="${2:-}"; shift 2 ;;
    --keep) KEEP=1; shift ;;
    *) echo "usage: $0 --old <sigil> --new <sigil> --aeon-ref <dir> --out <dir> [--fr1 <dir>] [--keep]" >&2
       echo "WALK_PROOF_END rc=2"; exit 2 ;;
  esac
done

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRATCH="$HERE/.scratch-walk-$$-$(date +%s)"
cleanup() { rm -rf "$SCRATCH"; }
trap cleanup EXIT
die() { echo "COULD NOT RUN ${1}: ${2}"; echo "WALK_PROOF_END rc=2"; exit 2; }

for v in OLD NEW REF OUT; do [ -n "${!v}" ] || die setup "missing --$(echo "$v" | tr A-Z a-z)"; done
[ -x "$OLD" ] || die setup "old binary not executable: $OLD"
[ -x "$NEW" ] || die setup "new binary not executable: $NEW"
[ -d "$REF" ] || die setup "aeon reference tree missing: $REF"
OLD="$(cd "$(dirname "$OLD")" && pwd)/$(basename "$OLD")"
NEW="$(cd "$(dirname "$NEW")" && pwd)/$(basename "$NEW")"
mkdir -p "$OUT" || die setup "cannot create $OUT"
OUT="$(cd "$OUT" && pwd)"
PY="$HERE/scripts/walk_inplace_proof.py"
[ -f "$PY" ] || die setup "$PY missing"

# the keystone alias, read from source
NATIVE="$HERE/crates/sigil-harness/src/native.rs"
grep -A2 '^pub fn config_a_keystones_flipped_profile() -> GameProfile {' "$NATIVE" | grep -q '^    config_a_profile()$' \
  || die setup "config_a_keystones_flipped_profile is no longer config_a_profile(); give it its own row"

# name|sigil build flags|frozen table the OLD walk read
SHAPES=(
  "plain||s4.txt"
  "debug|--debug|s4_debug.txt"
  "demo|--game demo|demo.txt"
  "demo_debug|--game demo --debug|demo_debug.txt"
  "config_a|--config-a|config_a.txt"
  "config_b|--config-b|config_b.txt"
  "lean|--lean|lean.txt"
  "stress_evict|--stress-evict|s4_debug.txt"
  "stress_art|--stress-art|s4_debug.txt"
)
for row in "${SHAPES[@]}"; do
  t="${row##*|}"
  grep -q "(\"$t\", include_str!" "$NATIVE" || die setup "frozen table $t is not in FROZEN_TABLES"
done

mkdir -p "$SCRATCH" || die setup "cannot create $SCRATCH"
cp -a "$REF" "$SCRATCH/old-tree" || die setup "copy of the reference for old failed"
cp -a "$REF" "$SCRATCH/new-tree" || die setup "copy of the reference for new failed"
echo "# old  $OLD ($("$OLD" --version 2>&1 | head -1))"
echo "# new  $NEW ($("$NEW" --version 2>&1 | head -1))"
echo "# aeon $REF ($(git -C "$REF" rev-parse HEAD 2>/dev/null || echo 'no git'))"

total=${#SHAPES[@]}
ok=0 bad=0
for row in "${SHAPES[@]}"; do
  name="${row%%|*}"; rest="${row#*|}"; flags="${rest%|*}"; table="${rest##*|}"
  for side in old new; do
    bin="$OLD"; [ "$side" = new ] && bin="$NEW"
    stem="$SCRATCH/$name.$side"
    # shellcheck disable=SC2086
    "$bin" build --aeon "$SCRATCH/$side-tree" $flags -o "$stem.bin" --emit-lst "$stem.lst" >"$stem.log" 2>&1
    rc=$?
    [ "$rc" -eq 0 ] || { tail -5 "$stem.log" >&2; die "$name" "$side build exited $rc"; }
    for suf in bin lst build.json; do [ -s "$stem.$suf" ] || die "$name" "$side produced no .$suf"; done
  done
  python3 -I "$PY" --shape "$name" --old "$SCRATCH/$name.old" --new "$SCRATCH/$name.new" \
    --tree "$SCRATCH/new-tree" --sigil "$HERE" --table "$table" --json "$OUT/$name.json" \
    >"$OUT/$name.proof.txt" 2>&1
  prc=$?
  verdict="$(grep '^PROOF ' "$OUT/$name.proof.txt" | tail -1)"
  [ -n "$verdict" ] || { cat "$OUT/$name.proof.txt" >&2; die "$name" "the proof printed no verdict (exit $prc)"; }
  [ "$prc" -eq 2 ] && { echo "$verdict"; die "$name" "could not measure"; }
  echo "$verdict"
  if [ "$prc" -eq 0 ]; then ok=$((ok+1)); else bad=$((bad+1)); grep '  FAIL' "$OUT/$name.proof.txt" | head -5; fi
  if [ -n "$FR1" ]; then
    if [ -f "$FR1/exp_$name.bin" ]; then
      if cmp -s "$FR1/exp_$name.bin" "$SCRATCH/$name.new.bin"; then echo "FR1 $name SAME"; else echo "FR1 $name MISMATCH"; fi
    else
      echo "FR1 $name absent"
    fi
  fi
  if [ "$KEEP" -eq 1 ]; then
    mkdir -p "$OUT/roms"
    for side in old new; do for suf in bin lst build.json; do
      cp "$SCRATCH/$name.$side.$suf" "$OUT/roms/$name.$side.$suf"; done; done
  fi
  for side in old new; do rm -f "$SCRATCH/$name.$side".*; done
done

echo "WALK_PROOF $ok/$total EXPLAINED"
rc=0; [ "$ok" -eq "$total" ] || rc=1
echo "WALK_PROOF_END rc=$rc"
exit $rc
