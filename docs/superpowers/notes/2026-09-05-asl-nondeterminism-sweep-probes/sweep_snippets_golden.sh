#!/usr/bin/env bash
# ASL-ORACLE-NONDETERMINISM — sweep 2: the golden snippet corpus.
#
# `crates/sigil-frontend-as/tests/snippets_golden.txt` holds 223 blocks of
# FROZEN BYTES minted from real asl by `gen_snippet_vectors`. Those are the
# vectors most at risk from an unstable asl shape, because an unstable value is
# already banked there and a CI test compares against it forever.
#
# Method: re-run the minting tool N times, snapshot the file it writes after
# each run, and compare the snapshots to EACH OTHER and to the COMMITTED file.
# The expected value is the committed artifact — not a value this sweep
# computes — so the comparison is not derived from the code under test.
#
# The tool rewrites the golden IN PLACE, so the file is restored from git after
# every run and again at the end.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"   # docs/superpowers/notes/<this dir>/ → repo root
GOLDEN="$REPO/crates/sigil-frontend-as/tests/snippets_golden.txt"
ASLDIR="${ASLDIR:-/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64}"
N="${1:-5}"
# Scratch lives OUTSIDE the repo tree: $HERE is a committed probe directory and
# is itself swept by `sweep_probes.sh`, so snapshots dropped beside this script
# would become corpus.
SCRATCH="${TMPDIR:-/tmp}/asl_golden_sweep.$$"
SNAP="$SCRATCH/snaps"
mkdir -p "$SNAP" "$SCRATCH/gentmp"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO/.cargo-target}"
export TMPDIR="$SCRATCH/gentmp"

BIN="$CARGO_TARGET_DIR/debug/gen_snippet_vectors"
[[ -x $BIN ]] || { echo "FATAL: build gen-snippet-vectors first" >&2; exit 2; }

echo "# asl    $ASLDIR/asl   md5 $(md5sum "$ASLDIR/asl" | cut -d' ' -f1)"
echo "# p2bin  $ASLDIR/p2bin md5 $(md5sum "$ASLDIR/p2bin" | cut -d' ' -f1)"
echo "# committed golden md5 $(git -C "$REPO" show HEAD:crates/sigil-frontend-as/tests/snippets_golden.txt | md5sum | cut -d' ' -f1)"
echo "# blocks $(grep -c '^=== ' "$GOLDEN")   N=$N"

# the committed baseline, taken from git, never from the working tree
git -C "$REPO" show HEAD:crates/sigil-frontend-as/tests/snippets_golden.txt > "$SNAP/committed.txt"

# ONE exit trap: the golden file must come back even on Ctrl-C, and the scratch
# tree goes with it. A second `trap ... EXIT` would silently replace this.
restore() { git -C "$REPO" checkout -- crates/sigil-frontend-as/tests/snippets_golden.txt; }
cleanup() { restore; rm -rf "$SCRATCH"; }
trap cleanup EXIT

# ── THE INJECTED CONTROL ─────────────────────────────────────────────────────
# `INJECT=1` appends one extra block whose asl output is KNOWN to vary run to
# run while asl still exits 0 with no diagnostic (`#f(<register>)` with `f` a
# defined `function`). A sweep that reports this block STABLE is a broken sweep,
# not a clean corpus. The control only varies under the s2disasm build of asl;
# under the s1disasm build that minted the goldens the same block is a constant
# $03C7, which is itself the point — the control must be run against the binary
# where the shape is live, or it proves nothing.
INJECT="${INJECT:-0}"
inject_block() {
    cat >> "$GOLDEN" <<'EOF'
=== zz_injected_instability_control ===
        cpu 68000
        phase 0
f       function p,$3C7
        move.w  #f(a1),d0
--- bytes ---
00 00 00 00
EOF
}

for i in $(seq 1 "$N"); do
    restore
    [[ $INJECT == 1 ]] && inject_block
    if ! ASL_BIN="$ASLDIR/asl" AS_MSGPATH="$ASLDIR" timeout 600 "$BIN" >"$SNAP/run$i.log" 2>&1; then
        echo "run $i: gen-snippet-vectors FAILED (exit $?) — see $SNAP/run$i.log"
        tail -5 "$SNAP/run$i.log"
        continue
    fi
    cp "$GOLDEN" "$SNAP/run$i.txt"
    printf 'run %-3s md5 %s\n' "$i" "$(md5sum "$SNAP/run$i.txt" | cut -d' ' -f1)"
done
restore

echo
echo "=== SWEEP 2: per-block verdict ==="
# Compare every produced snapshot against run1 and against the committed file,
# block by block, so an unstable block is NAMED rather than just counted.
python3 - "$SNAP" "$N" <<'PY'
import sys, os, re, collections
snap, n = sys.argv[1], int(sys.argv[2])

def blocks(path):
    out, name, byts = {}, None, None
    inb = False
    for line in open(path):
        m = re.match(r'^=== (.*) ===$', line.rstrip('\n'))
        if m:
            if name is not None: out[name] = byts
            name, byts, inb = m.group(1), None, False
        elif line.strip() == '--- bytes ---':
            inb = True
        elif inb and byts is None:
            byts = line.strip()
    if name is not None: out[name] = byts
    return out

runs = []
for i in range(1, n+1):
    p = os.path.join(snap, f'run{i}.txt')
    if os.path.exists(p): runs.append((i, blocks(p)))
committed = blocks(os.path.join(snap, 'committed.txt'))

if not runs:
    print('NO RUNS PRODUCED — sweep is unmeasurable, not green'); sys.exit(3)

# Iterate the union so an INJECTED control block (absent from the committed
# file) is judged too — otherwise the control could never be reported.
names = list(committed.keys())
for nm in runs[0][1]:
    if nm not in committed:
        names.append(nm)
unstable, mismatch = [], []
for nm in names:
    vals = {}
    for i, b in runs:
        vals.setdefault(b.get(nm), []).append(i)
    if len(vals) > 1:
        first_varied = min(min(v) for k, v in vals.items() if v != vals[list(vals)[0]] or True)
        # run index at which a value first differed from run 1's value
        base = runs[0][1].get(nm)
        fv = next((i for i, b in runs if b.get(nm) != base), None)
        unstable.append((nm, fv, list(vals.keys())))
    if nm in committed and runs[0][1].get(nm) != committed.get(nm):
        mismatch.append((nm, committed.get(nm), runs[0][1].get(nm)))

print(f'blocks in committed golden : {len(names)}')
print(f'minting runs completed     : {len(runs)} of {n}')
print(f'blocks UNSTABLE across runs: {len(unstable)}')
print(f'blocks differing from the COMMITTED value (run 1): {len(mismatch)}')
for nm, fv, vals in unstable:
    print(f'  UNSTABLE {nm}  first varied at run {fv}  values={vals}')
for nm, c, r in mismatch[:40]:
    print(f'  DIFFERS-FROM-COMMITTED {nm}\n      committed: {c}\n      minted   : {r}')
if len(mismatch) > 40:
    print(f'  ... and {len(mismatch)-40} more')
PY
