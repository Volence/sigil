#!/usr/bin/env bash
# ASL-ORACLE-NONDETERMINISM — sweep 2b: the OTHER two asl-minted golden corpora.
#
# `crates/sigil-isa/tests/m68k_golden_vectors.txt` and `z80_golden_vectors.txt`
# are frozen bytes minted from real asl by `gen-m68k-vectors` / `gen-z80-vectors`.
# The parcel brief named only `snippets_golden.txt`; these two are the same class
# of artifact and the same risk, so they are swept the same way: re-mint N times,
# compare the snapshots to each other AND to the committed file taken from git.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"   # docs/superpowers/notes/<this dir>/ → repo root
ASLDIR="${ASLDIR:-/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64}"
N="${1:-20}"
# Scratch outside the repo tree — $HERE is a committed probe directory that
# `sweep_probes.sh` itself sweeps.
SCRATCH="${TMPDIR:-/tmp}/asl_isa_sweep.$$"
mkdir -p "$SCRATCH"
trap 'rm -rf "$SCRATCH"' EXIT
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO/.cargo-target}"
export TMPDIR="$SCRATCH"

echo "# asl $ASLDIR/asl md5 $(md5sum "$ASLDIR/asl" | cut -d' ' -f1)  N=$N"

sweep_one() {
    local bin="$1" rel="$2"
    local abs="$REPO/$rel"
    local B="$CARGO_TARGET_DIR/debug/$bin"
    [[ -x $B ]] || { echo "FATAL: $bin not built"; return 2; }
    local committed; committed="$(git -C "$REPO" show "HEAD:$rel" | md5sum | cut -d' ' -f1)"
    echo "--- $rel (committed md5 $committed, $(git -C "$REPO" show "HEAD:$rel" | wc -l) lines)"
    local seen="" fails=0
    for i in $(seq 1 "$N"); do
        git -C "$REPO" checkout -- "$rel"
        if ! ASL_BIN="$ASLDIR/asl" AS_MSGPATH="$ASLDIR" timeout 600 "$B" >/dev/null 2>&1; then
            echo "   run $i: generator FAILED"; fails=$((fails+1)); continue
        fi
        seen="$seen $(md5sum "$abs" | cut -d' ' -f1)"
    done
    git -C "$REPO" checkout -- "$rel"
    local u; u="$(printf '%s\n' $seen | tr -s ' ' '\n' | grep -v '^$' | sort -u | wc -l)"
    local runs; runs="$(printf '%s\n' $seen | tr -s ' ' '\n' | grep -vc '^$')"
    local one; one="$(printf '%s\n' $seen | tr -s ' ' '\n' | grep -v '^$' | head -1)"
    if [[ $runs -eq 0 ]]; then
        echo "   NO RUNS PRODUCED — unmeasurable, not green (generator failed $fails times)"
    elif [[ $u -eq 1 && $one == "$committed" ]]; then
        echo "   STABLE across $runs runs AND identical to the committed file ($one)"
    elif [[ $u -eq 1 ]]; then
        echo "   STABLE across $runs runs but DIFFERS from committed: minted $one vs committed $committed"
    else
        echo "   UNSTABLE — $u distinct results across $runs runs:"
        printf '%s\n' $seen | tr -s ' ' '\n' | grep -v '^$' | sort | uniq -c
    fi
}

sweep_one gen-m68k-vectors crates/sigil-isa/tests/m68k_golden_vectors.txt
sweep_one gen-z80-vectors  crates/sigil-isa/tests/z80_golden_vectors.txt
