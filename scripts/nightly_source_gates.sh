#!/usr/bin/env bash
# Standing backstop for sigil's AEON-SOURCE-DERIVED gates.
#
# WHY THE TRIGGER IS A CLOCK AND NOT A REFREEZE. The warn-tier corpus and its
# neighbours read aeon SOURCE through AEON_DIR, but nobody ran them against a
# fresh aeon tip except at refreeze time — and a refreeze happens only when ROM
# bytes move. Six consecutive zero-byte aeon parcels therefore hid a real
# `layout.odd-field` finding for a day. A ritual keyed to byte movement is
# structurally blind to a source-derived lint set moving, so this lane runs on a
# clock instead and covers every source-derived check without aeon's ritual
# having to enumerate them. Diagnosis:
# docs/superpowers/notes/2026-08-22-warn-tier-drift-open.md
#
# Both ends are DETACHED checkouts of committed master tips, outside their repo
# roots. Outside, for the same reason aeon's nightly gives: a worktree under the
# repo root double-counts every module in aeon's tools/emp_helper_closure.py tree
# scan. Detached and committed, so the lane's verdict is about master-vs-master —
# a session's uncommitted work in either checkout cannot colour it, and the aeon
# main tree carries the owner's live content edits at all times.
#
# Exit-code contract, mirroring aeon/tools/nightly_effects_gates.sh:
#   0  every gate passed
#   1  a gate FAILED
#   2  the lane COULD NOT RUN
# Both nonzero cases notify. A backstop that silently cannot run is the
# vacuous-gate pattern this exists to prevent.
#
# --selftest-fail exercises the notification path without running anything.
set -uo pipefail

SIGIL_MAIN=/home/volence/sonic_hacks/sigil
AEON_MAIN=/home/volence/sonic_hacks/aeon
SIGIL_GATES=/home/volence/sonic_hacks/.sigil-source-gates
AEON_GATES=/home/volence/sonic_hacks/.aeon-sigil-gates
STATE=${XDG_STATE_HOME:-$HOME/.local/state}/sigil-source-gates
LOG="$STATE/nightly.log"
mkdir -p "$STATE"

# On disk, never under /tmp: /tmp is tmpfs here and a cargo build there wedges
# the shell.
export CARGO_TARGET_DIR=/home/volence/sonic_hacks/.sigil-source-gates-target

# THE GATES THIS LANE RUNS: every sigil test binary whose inputs are aeon SOURCE
# plus sigil's own compilation of it. Nothing here reads a built aeon ROM, a
# listing, or a golden CRC — those need `./build.sh` to have run in the aeon tree
# and belong to the artifact lane, deliberately not this one (see EXCLUDED below).
SOURCE_GATES=(
    # the warn tier over the real corpus — the gate the ruling is about
    warn_tier_corpus
    # whole-corpus source analyses
    contract_closure_corpus
    dead_save_corpus
    movem_restore_guard_corpus
    out_verify_corpus
    preserves_corpus
    slot_type_corpus
    cfg_blind_spots
    # negative probes: doctor an aeon source file, require the compiler to object
    core_negative_probes
    dplc_negative_probes
    hblank_negative_probes
    mt_negative_probes
    sfx_negative_probes
    tranche2_negative_probes
    tranche5_negative_probes
    tranche6_negative_probes
    tranche7_negative_probes
    tranche24_spelling_probes
    z80_clobbers_incomplete
    # source-derived drift and derivation gates
    act_fixture_drift
    banked_carrier_drift
    p5_constants_flip
    parcel_8b_stage_gen_touchers
    seam2_layout_derivation
    structs_module
    # source-only compile and round-trip oracles
    dac_port
    diag_assert_vector
    game_debug_port
    m68k_roundtrip_stream
    native_object_bank_budget
    seam2_colink_probe
    seam2_phased_head
    subcommands
)

# DELIBERATELY EXCLUDED, and why. These read bytes that only exist after a build:
#   - the ~63 `*_port` region diffs, m1b_gate, m1c_vector_table and
#     repin_pins::pins_rs_is_current read $AEON_DIR/s4.bin / s4.debug.bin / s4.lst;
#   - the ~18 golden-CRC gates (boot_port, native_full_rom, the seam2 co-links,
#     math_port, mt_port, sfx_port, …) compare against
#     crates/sigil-harness/golden/*.bin and provenance.toml.
# A source-drift lane that also had to build four ROM shapes would take an order
# of magnitude longer and would go red on aeon build breakage rather than on the
# thing it watches. Those gates have their own trigger: aeon's byte-identity
# ritual, which fires exactly when bytes move — which is the correct trigger for
# them and the wrong one for these.

note() {
    echo "$(date -Is) $1" >> "$LOG"
    notify-send -u critical "sigil source gates" "$1" 2>/dev/null || true
}

if [[ ${1:-} == --selftest-fail ]]; then
    note "SELFTEST: the failure-notification path works"
    exit 1
fi

for d in "$SIGIL_MAIN" "$AEON_MAIN"; do
    [[ -d "$d/.git" || -f "$d/.git" ]] \
        || { note "COULD NOT RUN: no repo at $d"; exit 2; }
done

# Detached checkouts at each repo's master tip.
if [[ ! -d "$SIGIL_GATES" ]]; then
    git -C "$SIGIL_MAIN" worktree add --detach "$SIGIL_GATES" master >> "$LOG" 2>&1 \
        || { note "COULD NOT RUN: sigil gates worktree creation failed"; exit 2; }
fi
if [[ ! -d "$AEON_GATES" ]]; then
    git -C "$AEON_MAIN" worktree add --detach "$AEON_GATES" master >> "$LOG" 2>&1 \
        || { note "COULD NOT RUN: aeon gates worktree creation failed"; exit 2; }
fi

# Both default to master and the timer never sets them. They exist so a branch can
# be put through the real lane before it lands, and so the lane's own red-first
# proof can point at a committed throwaway state.
SIGIL_REF=${SIGIL_SOURCE_GATES_REF:-master}
AEON_REF=${AEON_SOURCE_GATES_REF:-master}

SIGIL_SHA=$(git -C "$SIGIL_MAIN" rev-parse "$SIGIL_REF") \
    || { note "COULD NOT RUN: cannot resolve sigil $SIGIL_REF"; exit 2; }
AEON_SHA=$(git -C "$AEON_MAIN" rev-parse "$AEON_REF") \
    || { note "COULD NOT RUN: cannot resolve aeon $AEON_REF"; exit 2; }
AT="sigil ${SIGIL_SHA:0:8} / aeon ${AEON_SHA:0:8}"

git -C "$SIGIL_GATES" checkout --force --detach "$SIGIL_SHA" >> "$LOG" 2>&1 \
    || { note "COULD NOT RUN: sigil checkout of $SIGIL_SHA failed"; exit 2; }
git -C "$AEON_GATES" checkout --force --detach "$AEON_SHA" >> "$LOG" 2>&1 \
    || { note "COULD NOT RUN: aeon checkout of $AEON_SHA failed"; exit 2; }
# The seam-emitted sound blobs are regenerated by the harness itself; the
# compression self-test vectors are not, and the corpus embeds them, so a
# checkout that has never been built cannot lower the sonic4 debug shape without
# this. Both the packer and the generator are source-only — no ROM is built here.
#
# Deleted first, not merely regenerated. These directories are gitignored, so
# `checkout --force` leaves whatever a previous night left behind, and a stale
# generated file is byte-indistinguishable from a correct one.
rm -rf "$AEON_GATES/engine/sound/generated" "$AEON_GATES/engine/debug/generated"
# And no built ROM or listing survives in this checkout. Nothing the lane runs reads
# one, and keeping the tree provably source-only is what makes that claim checkable
# rather than asserted: a leftover s4.bin from a hand-run build is byte-identical in
# appearance to a current one, and several port tests treat its mere PRESENCE as
# "there is an aeon tree here".
rm -f "$AEON_GATES"/*.bin "$AEON_GATES"/*.lst "$AEON_GATES"/*.p "$AEON_GATES"/*.h

if [[ ! -x "$AEON_GATES/tools/bin/salvador" ]]; then
    mkdir -p "$AEON_GATES/tools/bin"
    make -C "$AEON_GATES/tools/salvador" -s > "$STATE/prepare.log" 2>&1 \
        || { note "COULD NOT RUN: salvador build failed at $AT — see $STATE/prepare.log"; exit 2; }
    cp "$AEON_GATES/tools/salvador/salvador" "$AEON_GATES/tools/bin/salvador" \
        || { note "COULD NOT RUN: salvador install failed at $AT"; exit 2; }
fi
( cd "$AEON_GATES" && python3 tools/gen_compression_vectors.py ) >> "$STATE/prepare.log" 2>&1 \
    || { note "COULD NOT RUN: compression vectors at $AT — see $STATE/prepare.log"; exit 2; }
[[ -f "$AEON_GATES/engine/debug/generated/compression_vectors.emp" ]] \
    || { note "COULD NOT RUN: gen_compression_vectors.py produced nothing at $AT"; exit 2; }

export AEON_DIR="$AEON_GATES"
# Strict: a reference path this lane cannot find HARD-FAILS instead of skipping
# green. A gate that skips reports nothing and reads as coverage.
export SIGIL_STRICT_GATE=1

ARGS=()
for g in "${SOURCE_GATES[@]}"; do ARGS+=(--test "$g"); done

OUT="$STATE/gates.log"
( cd "$SIGIL_GATES" && cargo test --release --workspace --no-fail-fast "${ARGS[@]}" \
    -- --nocapture ) > "$OUT" 2>&1
rc=$?

# Every named gate must have produced a result line. A cargo invocation that
# silently selected nothing exits 0, and that is indistinguishable from a green
# run by exit code alone.
binaries=$(grep -c '^test result:' "$OUT")
if (( binaries != ${#SOURCE_GATES[@]} )); then
    note "COULD NOT RUN: ${#SOURCE_GATES[@]} gates named but $binaries ran at $AT — see $OUT"
    exit 2
fi
passed=$(awk '/^test result:/ {p += $4} END {print p+0}' "$OUT")
failed=$(awk '/^test result:/ {f += $6} END {print f+0}' "$OUT")
if (( passed == 0 )); then
    note "COULD NOT RUN: no test executed at $AT — see $OUT"
    exit 2
fi
# A `skip:` line is a reference gate that measured nothing while reporting green.
# SIGIL_STRICT_GATE should make these impossible; if one appears, the strict path
# has a hole and the run's green means less than it reads.
if grep -q 'skip:' "$OUT"; then
    note "COULD NOT RUN: $(grep -c 'skip:' "$OUT") gate(s) SKIPPED under strict at $AT — see $OUT"
    exit 2
fi

# The open-findings register, named in ordinary output so its rows are read
# nightly rather than resting in a file nobody is obliged to open.
REGISTER=$(sed -n '/^open warn-tier findings:/,/^test /p' "$OUT" | grep -v '^test ' | head -20)

if (( rc == 0 && failed == 0 )); then
    {
        echo "$(date -Is) OK at $AT ($passed passed, $binaries gates)"
        [[ -n "$REGISTER" ]] && echo "$REGISTER" | sed 's/^/    /'
    } >> "$LOG"
    exit 0
fi

names=$(awk '/^failures:$/ {f=1; next} /^test result:/ {f=0} f && /^    [a-z]/ {print $1}' "$OUT" \
    | sort -u | tr '\n' ' ')
note "SOURCE GATES FAILED at $AT — $failed failed / $passed passed: $names (see $OUT)"
exit 1
