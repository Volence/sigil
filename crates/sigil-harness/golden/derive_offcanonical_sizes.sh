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
#   SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
#   AEON_DIR=/path/to/aeon ./derive_offcanonical_sizes.sh
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SIGIL_ROOT="$(cd "$HERE/../../.." && pwd)"
AEON="${AEON_DIR:-/home/volence/sonic_hacks/aeon}"
[[ -d "$AEON" ]] || { echo "ERROR: AEON_DIR not a dir: $AEON"; exit 1; }
if [[ -z "${SIGIL_EMIT:-}" || ! -x "${SIGIL_EMIT:-}" ]]; then
    echo "ERROR: set SIGIL_EMIT to <sigil>/target/release/emit_sound_blob (sound-on builds need it)."
    exit 1
fi

echo "== Stage-3 P4a — sigil-native off-canonical size derivation (asl-free) =="
echo "   aeon: $AEON  ($(cd "$AEON" && git rev-parse --short HEAD 2>/dev/null || echo '?'))"

( cd "$SIGIL_ROOT" && cargo build --release -p sigil-harness --bin derive_offcanon )
AEON_DIR="$AEON" SIGIL_EMIT="$SIGIL_EMIT" \
    "$SIGIL_ROOT/target/release/derive_offcanon" "$HERE/offcanonical_sizes"

echo "== done — tables in $HERE/offcanonical_sizes (commit them; sigil-native provenance) =="
