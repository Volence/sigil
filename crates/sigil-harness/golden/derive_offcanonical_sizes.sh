#!/bin/bash
# derive_offcanonical_sizes.sh — Stage-3 P4a: re-derive the four off-canonical size
# tables from SIGIL'S OWN resolved layout, retiring the asl-`.lst` parse.
#
# The predecessor (`capture_offcanonical_sizes.sh` + `capture_offcanon`) built each
# target through asl and parsed its `.lst` `Symbol Table` for the boundary addresses —
# the LAST asl-derived constants in the system. This script instead drives the native
# derivation (`derive_offcanon` → `native::derive_frozen_table`), which resolves the
# frozen chain in-process and reads each label's ROM LMA off the resolved sections
# (`section.lma + label.offset`) — LMA-correct for the phased z80 idle and synthesizing
# the section-END markers from section geometry. No asl, no listing file, no aeon build.
#
# The re-derived addresses are proven == the committed tables by
# `native_offcanonical_placement::*_size_table_rederives_native`. They remain golden
# provenance: on any ruled post-flip golden re-baseline they re-derive from sigil (see
# `crates/sigil-harness/golden/PROVENANCE.md` and the S1.2 size-capture handoff note).
#
# Usage:
#   CARGO_TARGET_DIR=<a build dir of this run's own> \
#   SIGIL_EMIT=$CARGO_TARGET_DIR/release/emit_sound_blob \
#   AEON_DIR=/path/to/aeon ./derive_offcanonical_sizes.sh
#
#   CARGO_TARGET_DIR is required, not defaulted: see THE BUILD DIRECTORY below.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SIGIL_ROOT="$(cd "$HERE/../../.." && pwd)"

# ── THE BUILD DIRECTORY, FIRST — never reached for in a shared one ───────────────────
# First because it costs nothing to ask and a run that cannot name the directory it
# builds into is not worth resolving a reference tree for; the refusal is therefore
# reachable with no aeon provisioned at all.
#
# The predecessor was `TARGET_DIR="${CARGO_TARGET_DIR:-$SIGIL_ROOT/target}"`, so a bare
# run built `derive_offcanon` into the shared checkout's `target/` and ran whatever was
# there. These tables are GOLDEN PROVENANCE — committed, and re-derived only on a ruled
# re-baseline — so a foreign or stale binary here does not fail, it produces frozen
# expectations attributed to the wrong assembler. There is no path this script could
# choose unasked that it could name honestly afterwards, so it chooses none (d-18).
if [[ -z "${CARGO_TARGET_DIR:-}" ]]; then
    echo "ERROR: this derivation has no build directory, and it will not pick one." >&2
    echo "       consulted  CARGO_TARGET_DIR   (unset)" >&2
    echo "       declined   $SIGIL_ROOT/target" >&2
    echo "                  — a checkout's shared target/, written and relinked by" >&2
    echo "                    whichever lane last built there. The size tables are golden" >&2
    echo "                    provenance: derived with a foreign binary they do not fail," >&2
    echo "                    they freeze wrong expectations under this tree's name." >&2
    echo "       Set CARGO_TARGET_DIR to a build directory of this run's own." >&2
    exit 1
fi
TARGET_DIR="$CARGO_TARGET_DIR"

# ── THE REFERENCE TREE ───────────────────────────────────────────────────────────────
# `scripts/lib/suite_paths.sh` implements the one precedence (contract/SUITE_PATHS.md):
# explicit AEON_DIR, then EMPYREAN_SUITE_ROOT/aeon, then the sibling derived from this
# checkout's own `git --git-common-dir`, then a refusal that names all of them. The
# include's path is computed from $0's directory, never from $PWD — this script is run
# from the directory it lives in.
#
# WHEN THE INCLUDE IS NOT REACHABLE, this script is a COPY planted outside the repo. Only
# step 1 is available there; a copy that names no checkout is refused rather than sent to
# a live working tree by a literal nobody chose.
SUITE_PATHS="$SIGIL_ROOT/scripts/lib/suite_paths.sh"
if [[ -r "$SUITE_PATHS" ]]; then
    # shellcheck source=../../../scripts/lib/suite_paths.sh
    source "$SUITE_PATHS"
    AEON=$(suite_resolve_checkout aeon AEON_DIR) || exit $?
elif [[ -n "${AEON_DIR:-}" ]]; then
    AEON="${AEON_DIR}"
    printf '# AEON_DIR=%s (step 1: explicit AEON_DIR; %s is not reachable from this copy)\n' \
        "$AEON" "$SUITE_PATHS" >&2
else
    printf 'suite-paths: REFUSING — cannot locate the aeon checkout.\n' >&2
    printf '       consulted  AEON_DIR              (unset)\n' >&2
    printf '       tried      %s   (not readable, so the full precedence is unavailable)\n' "$SUITE_PATHS" >&2
    printf '       Export AEON_DIR to the aeon checkout. This does NOT fall back to a live\n' >&2
    printf '       working tree: a derivation against a tree nobody named is one nobody can\n' >&2
    printf '       reproduce.\n' >&2
    exit 3
fi

[[ -d "$AEON" ]] || { echo "ERROR: AEON_DIR not a dir: $AEON"; exit 1; }
if [[ -z "${SIGIL_EMIT:-}" || ! -x "${SIGIL_EMIT:-}" ]]; then
    echo "ERROR: set SIGIL_EMIT to <a build dir of this run's own>/release/emit_sound_blob (sound-on builds need it)."
    exit 1
fi

echo "== Stage-3 P4a — sigil-native off-canonical size derivation (asl-free) =="
echo "   aeon: $AEON  ($(cd "$AEON" && git rev-parse --short HEAD 2>/dev/null || echo '?'))"

# TARGET_DIR was resolved at the top of this script, before anything was reached for.
# Passed explicitly rather than inherited, so the directory the build writes into and
# the one the next line reads from are one statement.
( cd "$SIGIL_ROOT" && CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release -p sigil-harness --bin derive_offcanon )
AEON_DIR="$AEON" SIGIL_EMIT="$SIGIL_EMIT" \
    "$TARGET_DIR/release/derive_offcanon" "$HERE/offcanonical_sizes"

echo "== done — tables in $HERE/offcanonical_sizes (commit them; sigil-native provenance) =="
