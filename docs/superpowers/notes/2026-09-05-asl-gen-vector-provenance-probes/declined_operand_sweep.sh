#!/usr/bin/env bash
# ASL-GEN-VECTORS-UNIDENTIFIED — the DECLINED-OPERAND enumeration over the three
# asl-minted golden corpora.
#
# The digest sweeps next door answer "which build minted this". They are BLIND to
# a different defect: a golden entry that quotes a byte column for an operand asl
# DECLINED to value. Under the reference build (md5 61e672562465725a8c102288a7da9098)
# such an entry is stable — it echoes the last value that build computed — so
# re-minting agrees with itself and the entry reads like a measurement.
#
# THE PARAMETER THIS SWEEP ENUMERATES BY: cross-build disagreement.
# The varying build (md5 0dee1f98e6480a4783d27ffd8b90896f) fills a declined
# operand from uninitialized memory instead, so it answers differently on every
# run. Therefore:
#
#     entry differs between the two builds, OR varies across N varying-build runs
#         => that entry carries an operand asl declined to value
#
# Both directions matter. A shape declined with nothing accepted above it reads
# 0000 on the reference build and random on the varying build (the cross-build
# diff catches it); a shape declined after an accepted value echoes that value on
# the reference build and random on the varying build (both checks catch it).
#
# This is ONE parameter and it is not the only one. It cannot see a shape both
# builds decline identically. `diagnostics_sweep.sh` beside this file is the
# independent second parameter (did asl SAY anything), and the corpus grep in the
# note is the third.
#
# The generators rewrite their goldens IN PLACE. Every run is followed by a
# restore from git, the restore is VERIFIED, and the sweep refuses to start if
# the three golden paths are not already clean — a comparison against a file a
# previous run left overwritten proves nothing.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"   # docs/superpowers/notes/<this dir>/ -> repo root
REF_DIR="${REF_DIR:-/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64}"
VAR_DIR="${VAR_DIR:-/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64}"
REF_MD5=61e672562465725a8c102288a7da9098
VAR_MD5=0dee1f98e6480a4783d27ffd8b90896f
N="${1:-4}"

SCRATCH="${TMPDIR:-/tmp}/asl_declined_sweep.$$"
mkdir -p "$SCRATCH/snaps" "$SCRATCH/gentmp"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO/.cargo-target}"
export TMPDIR="$SCRATCH/gentmp"

GOLDENS=(
    crates/sigil-isa/tests/m68k_golden_vectors.txt
    crates/sigil-isa/tests/z80_golden_vectors.txt
    crates/sigil-frontend-as/tests/snippets_golden.txt
)

restore_all() { git -C "$REPO" checkout -- "${GOLDENS[@]}" 2>/dev/null; }
cleanup() { restore_all; rm -rf "$SCRATCH"; }
trap cleanup EXIT

# ---- preconditions: identify both instruments, and refuse a dirty start -------
fail=0
for d in "$REF_DIR:$REF_MD5:reference" "$VAR_DIR:$VAR_MD5:varying"; do
    dir="${d%%:*}"; rest="${d#*:}"; want="${rest%%:*}"; role="${rest#*:}"
    if [[ ! -x $dir/asl ]]; then
        echo "FATAL: no asl at $dir ($role build)" >&2; fail=1; continue
    fi
    got="$(md5sum "$dir/asl" | cut -d' ' -f1)"
    printf '# %-9s asl %s md5 %s\n' "$role" "$dir/asl" "$got"
    [[ $got == "$want" ]] || { echo "FATAL: $role build md5 $got, wanted $want" >&2; fail=1; }
done
[[ $fail == 0 ]] || exit 3

dirty="$(git -C "$REPO" status --porcelain -- "${GOLDENS[@]}")"
if [[ -n $dirty ]]; then
    echo "FATAL: golden files are not clean at start; refusing to sweep:" >&2
    echo "$dirty" >&2
    exit 3
fi
echo "# goldens clean at start; N=$N varying-build runs per corpus"
echo

# ---- THE INJECTED CONTROL ----------------------------------------------------
# `INJECT=1` appends one block to the snippet corpus whose operand asl DECLINES
# to value while still exiting 0 with no diagnostic (`#f(<register>)`, `f` a
# defined `function`) — the silent class, the one a tool accepts and freezes.
# A sweep that reports this block clean is a BLIND sweep, not a clean corpus, so
# the control is how this instrument earns the right to report zero.
#
# The block is appended after every restore, because the restore would remove it.
INJECT="${INJECT:-0}"
SNIPPET_GOLDEN="$REPO/crates/sigil-frontend-as/tests/snippets_golden.txt"
inject_block() {
    [[ $INJECT == 1 ]] || return 0
    cat >> "$SNIPPET_GOLDEN" <<'EOF'
=== zz_injected_declined_operand_control ===
        cpu 68000
        phase 0
f       function p,$3C7
        move.w  #f(a1),d0
--- bytes ---
00 00 00 00
EOF
}

# ---- one corpus --------------------------------------------------------------
# $1 = cargo bin filename in $CARGO_TARGET_DIR/debug, $2 = repo-relative golden
sweep_one() {
    local bin="$1" rel="$2"
    local abs="$REPO/$rel" B="$CARGO_TARGET_DIR/debug/$bin"
    local tag; tag="$(basename "$rel" .txt)"
    echo "=== $rel"
    # only the snippet corpus can carry the control (the arrow-format corpora
    # take their snippets from Rust, not from the golden file)
    local inject=0
    [[ $INJECT == 1 && $rel == *snippets_golden.txt ]] && inject=1
    [[ $inject == 1 ]] && echo "   (INJECTED CONTROL ACTIVE — a clean verdict here would be a sweep defect)"
    if [[ ! -x $B ]]; then
        echo "   UNMEASURABLE: generator $bin not built at $B" >&2
        return 2
    fi

    git -C "$REPO" show "HEAD:$rel" > "$SCRATCH/snaps/$tag.committed"

    # one reference-build mint
    restore_all
    [[ $inject == 1 ]] && inject_block
    if ! ASL_BIN="$REF_DIR/asl" P2BIN_BIN="$REF_DIR/p2bin" AS_MSGPATH="$REF_DIR" \
         timeout 900 "$B" >"$SCRATCH/snaps/$tag.ref.log" 2>&1; then
        echo "   UNMEASURABLE: reference mint FAILED; see $SCRATCH/snaps/$tag.ref.log" >&2
        restore_all; return 2
    fi
    cp "$abs" "$SCRATCH/snaps/$tag.ref"
    restore_all
    verify_restore "$rel" || return 2

    # N varying-build mints
    local produced=0 i
    for i in $(seq 1 "$N"); do
        restore_all
        [[ $inject == 1 ]] && inject_block
        if ! ASL_BIN="$VAR_DIR/asl" P2BIN_BIN="$VAR_DIR/p2bin" AS_MSGPATH="$VAR_DIR" \
             timeout 900 "$B" >"$SCRATCH/snaps/$tag.var$i.log" 2>&1; then
            echo "   varying run $i: generator FAILED (see $SCRATCH/snaps/$tag.var$i.log)"
            continue
        fi
        cp "$abs" "$SCRATCH/snaps/$tag.var$i"
        produced=$((produced+1))
    done
    restore_all
    verify_restore "$rel" || return 2

    if [[ $produced -eq 0 ]]; then
        echo "   UNMEASURABLE: no varying-build mint completed — NOT a clean verdict"
        return 2
    fi
    python3 "$HERE/compare_entries.py" "$SCRATCH/snaps" "$tag" "$produced" "$rel"
}

# The restore is only real if it landed. A sweep that draws a verdict from a file
# the previous run overwrote is measuring itself.
verify_restore() {
    local rel="$1"
    local d; d="$(git -C "$REPO" status --porcelain -- "$rel")"
    if [[ -n $d ]]; then
        echo "   FATAL: restore of $rel did NOT land: $d" >&2
        return 1
    fi
    return 0
}

sweep_one gen-m68k-vectors      crates/sigil-isa/tests/m68k_golden_vectors.txt
echo
sweep_one gen-z80-vectors       crates/sigil-isa/tests/z80_golden_vectors.txt
echo
sweep_one gen_snippet_vectors   crates/sigil-frontend-as/tests/snippets_golden.txt
