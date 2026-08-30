#!/usr/bin/env bash
# The aeon-reference DRIFT job — step 1 of SIGIL-DECOUPLE.
#
# Byte identity stops being a landing blocker and becomes this: a nightly,
# NON-BLOCKING observation. The job still measures byte identity; it just stops
# stopping anyone. It is the instrument that produces step 4's answer — whether the
# byte-identity certification gets archived or promoted.
#
# NON-BLOCKING IS STRUCTURAL, NOT A PROMISE. Nothing in a landing path can reach this
# file: it is invoked only by `sigil-ref-drift.timer` (and by hand), it is named in no
# cargo test, no build script and no gate list, and it writes nothing into either
# repository. `drift_nightly_harness.rs` asserts those properties rather than trusting
# them, because "nothing calls it" is exactly the kind of claim that stops being true
# quietly.
#
# WHY IT IS BESIDE `nightly_source_gates.sh` AND NOT INSIDE IT. That lane's checkout is
# SOURCE-ONLY BY CONSTRUCTION — it scrubs `*.bin` and `*.lst` out of the reference tree
# on every run, and its own header states that an artifact-dependent run sharing that
# tree gets its ROMs deleted mid-suite, surfacing as ~127 golden mismatches rather than
# as a race. This job is artifact-dependent by definition: it builds ROMs and compares
# CRCs. The two lanes therefore need two checkouts, and a job with its own checkout,
# its own cadence and its own exit contract is a second script, not a section.
#
# EXIT CONTRACT. These are REPORTING codes. No landing consumes them; the timer's unit
# is the only reader:
#   0  QUIET            every measured shape matched a real expectation
#   1  DRIFT            a red. Settles step 4's question in a SINGLE observation.
#   2  NOTHING MEASURED no record, no reader, an unbuildable tree, a missing shape.
#                       Not a pass, not a zero, not green.
#   3  UNVERIFIED       a change was observed for which no expectation existed.
# Precedence when several apply: DRIFT > NOTHING MEASURED > UNVERIFIED > QUIET.
#
# --selftest-fail exercises the notification path without running anything.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SIGIL_MAIN=/home/volence/sonic_hacks/sigil
AEON_MAIN=/home/volence/sonic_hacks/aeon
# This job's OWN checkouts. Never the source-gate lane's (scrubbed source-only), never
# a peer's live working tree (mid-edit, and the owner authors in the aeon main tree).
SIGIL_DRIFT=/home/volence/sonic_hacks/.sigil-ref-drift
STATE=${XDG_STATE_HOME:-$HOME/.local/state}/sigil-ref-drift
LOG="$STATE/nightly.log"
mkdir -p "$STATE"

# On disk, never under /tmp: /tmp is tmpfs here and a cargo build there wedges the shell.
export CARGO_TARGET_DIR=/home/volence/sonic_hacks/.sigil-ref-drift-target

note() {
    echo "$(date -Is) $1" >> "$LOG"
    notify-send -u normal "sigil ref drift" "$1" 2>/dev/null || true
}

# NOTHING MEASURED still gets written down. A night that could not measure is a fact
# about the instrument, and leaving it out of the ledger would let the record of
# quiet nights read as continuous coverage.
record_unmeasured() {
    python3 "$HERE/drift_report.py" observe \
        --ledger "$DRIFT_LEDGER" \
        --aeon-rev "${AEON_SHA:-unknown}" \
        --sigil-linked-rev "${SIGIL_LINKED:-unknown}" \
        --sigil-closure-rev "${SIGIL_CLOSURE:-unknown}" \
        --sigil-tree-state "${SIGIL_TREE:-unknown}" \
        --observed-at "$(date -Is)" >> "$LOG" 2>&1
    note "NOTHING MEASURED: $1"
    exit 2
}

if [[ ${1:-} == --selftest-fail ]]; then
    note "SELFTEST: the notification path works"
    exit 2
fi

# ── N, read at RUN TIME ──────────────────────────────────────────────────────────
# N is the owner's number, ruled provisionally by the hub in his place. It lives in a
# config file so overturning it costs an edit, never a source change and never a
# parcel. There is NO fallback here and none in drift_report.py: an unreadable config
# means the job cannot say what it is counting toward, which is a state to report and
# not one to paper over with a default.
CONF="$HERE/drift-nightly.conf"
[[ -r "$CONF" ]] || { note "COULD NOT RUN: no readable config at $CONF, so N is unknown"; exit 2; }
# shellcheck source=/dev/null
source "$CONF" || { note "COULD NOT RUN: $CONF is not sourceable"; exit 2; }

N="${SIGIL_DRIFT_N:-${DRIFT_CHAIN_TARGET_N:-}}"
N_SOURCE="$CONF"
[[ -n "${SIGIL_DRIFT_N:-}" ]] && N_SOURCE="SIGIL_DRIFT_N (environment override)"
[[ "$N" =~ ^[1-9][0-9]*$ ]] \
    || { note "COULD NOT RUN: N is not a positive integer (got '${N:-unset}' from $N_SOURCE)"; exit 2; }

: "${DRIFT_LEDGER:?the config must set DRIFT_LEDGER}"
: "${DRIFT_AEON_TREE:?the config must set DRIFT_AEON_TREE}"
: "${DRIFT_SHAPES:?the config must set DRIFT_SHAPES}"

# ── the revisions ────────────────────────────────────────────────────────────────
for d in "$SIGIL_MAIN" "$AEON_MAIN"; do
    [[ -d "$d/.git" || -f "$d/.git" ]] || { note "COULD NOT RUN: no repo at $d"; exit 2; }
done

# PUBLISHED tips, read from the remote at measurement time. A revision that is only
# local can be orphaned by the next rebase, and a record keyed on a coordinate that
# stops existing describes nothing. A tracking ref is a cached answer, so both are
# fetched first.
git -C "$SIGIL_MAIN" fetch -q origin 2>>"$LOG"
git -C "$AEON_MAIN" fetch -q origin 2>>"$LOG"
SIGIL_SHA=$(git -C "$SIGIL_MAIN" rev-parse origin/master 2>>"$LOG") \
    || { note "COULD NOT RUN: cannot resolve sigil origin/master"; exit 2; }
AEON_SHA=$(git -C "$AEON_MAIN" rev-parse origin/master 2>>"$LOG") \
    || { note "COULD NOT RUN: cannot resolve aeon origin/master"; exit 2; }

if [[ ! -d "$SIGIL_DRIFT" ]]; then
    git -C "$SIGIL_MAIN" worktree add --detach "$SIGIL_DRIFT" "$SIGIL_SHA" >> "$LOG" 2>&1 \
        || { note "COULD NOT RUN: sigil drift worktree creation failed"; exit 2; }
fi
git -C "$SIGIL_DRIFT" checkout --force --detach "$SIGIL_SHA" >> "$LOG" 2>&1 \
    || { note "COULD NOT RUN: sigil checkout of $SIGIL_SHA failed"; exit 2; }

# ── the assembler, and the revision it was LINKED at ─────────────────────────────
# The assembler is a build input this repo does not pin: `SIGIL_BUILD`/`SIGIL_EMIT`
# come from the environment, and `SIGIL_EMIT` WRITES engine/sound/generated. A clean
# tracked tree at a fixed aeon revision can therefore build a different ROM with no
# cause visible in the tree at all. So the key names the binary that ran, asked of the
# binary itself — `git rev-parse HEAD` answers a different question and trailing HEAD
# after a docs-only commit is the normal steady state.
( cd "$SIGIL_DRIFT" && cargo build --release --bin sigil --bin emit_sound_blob ) \
    >> "$STATE/build.log" 2>&1 \
    || { note "COULD NOT RUN: the assembler did not build at sigil ${SIGIL_SHA:0:8} — see $STATE/build.log"; exit 2; }
SIGIL_BIN="$CARGO_TARGET_DIR/release/sigil"
VERSION_OUT=$("$SIGIL_BIN" --version 2>&1) \
    || { note "COULD NOT RUN: the built assembler cannot report its version"; exit 2; }
SIGIL_LINKED=$(sed -n 's/^  revision:  *\([0-9a-f]\{40\}\)$/\1/p' <<< "$VERSION_OUT")
SIGIL_CLOSURE=$(sed -n 's/^  closure-revision: *\([0-9a-f]\{40\}\).*$/\1/p' <<< "$VERSION_OUT")
SIGIL_TREE=$(sed -n 's/^  tree:  *\([a-z-]*\) at capture.*$/\1/p' <<< "$VERSION_OUT")
# Loud on unmeasurable: without a revision from the binary there is no honest key, and
# a key filled in from git HEAD would name a tree rather than the assembler that ran.
[[ -n "$SIGIL_LINKED" && -n "$SIGIL_CLOSURE" ]] \
    || { note "COULD NOT RUN: \`sigil --version\` did not report a linked revision and a \
closure revision, so this build has no honest key: $(head -3 <<< "$VERSION_OUT" | tr '\n' ' ')"; exit 2; }
SIGIL_TREE="${SIGIL_TREE:-unknown}"
AT="aeon ${AEON_SHA:0:8} / sigil ${SIGIL_CLOSURE:0:8} (linked ${SIGIL_LINKED:0:8}, tree $SIGIL_TREE)"

# ── the reference tree, at the engine lane's live tip ────────────────────────────
# provision-aeon-ref.sh carries the traps: 15 gitignored artifacts a bare
# `worktree add --detach` does not bring, whose absence reads as ~200 golden
# divergences. It derives from the revision whether its golden control applies, so
# provisioning at a live tip is a supported shape rather than a refusal.
BUILD_START=$(date +%s)
rm -f "$DRIFT_AEON_TREE"/*.bin "$DRIFT_AEON_TREE"/*.lst 2>/dev/null
SIGIL_BIN="$SIGIL_BIN" REF_TARGET="$CARGO_TARGET_DIR" AEON_REPO="$AEON_MAIN" \
    "$SIGIL_DRIFT/scripts/provision-aeon-ref.sh" "$DRIFT_AEON_TREE" "$AEON_SHA" \
    > "$STATE/provision.log" 2>&1
PROVISION_RC=$?

# ── measure ──────────────────────────────────────────────────────────────────────
# A shape is measured only from a file THIS RUN wrote. The provisioning step copies
# frozen reference ROMs into the tree before building, so an mtime older than the
# build start is a golden copy wearing a build's name — and measuring one would
# compare the record against itself.
SHAPE_ARGS=()
MEASURED=0
for spec in $DRIFT_SHAPES; do
    name="${spec%%:*}"; file="${spec#*:}"
    out=$(python3 - "$DRIFT_AEON_TREE/$file" "$BUILD_START" <<'PY'
import os, sys, zlib
p, start = sys.argv[1], int(sys.argv[2])
if not os.path.exists(p):
    print("unmeasured"); raise SystemExit
if os.path.getmtime(p) < start:
    print("unmeasured"); raise SystemExit
d = open(p, "rb").read()
print(f"{zlib.crc32(d) & 0xffffffff:08x}/{len(d)}")
PY
)
    SHAPE_ARGS+=(--shape "$name=$out")
    [[ "$out" != "unmeasured" ]] && MEASURED=$((MEASURED + 1))
done
if (( MEASURED == 0 )); then
    record_unmeasured "no shape was built at $AT (provisioning exited $PROVISION_RC) — see $STATE/provision.log"
fi

# ── the reference-path sweep ─────────────────────────────────────────────────────
# Source-only and independent of the record: it reports even on a night when nothing
# could be measured. The file classifier is EXTRACTED from nightly_source_gates.sh,
# which already answers the same question; a retyped copy drifts and the drift is
# invisible.
READS_PATTERN=$(sed -n "s/.*grep -rlE '\([^']*\)'.*/\1/p" "$SIGIL_DRIFT/scripts/nightly_source_gates.sh" | head -1)
if [[ -z "$READS_PATTERN" ]]; then
    SWEEP_OUT="CANNOT RUN: the reference-reading pattern could not be extracted from \
nightly_source_gates.sh, so the sweep does not know which files it covers"
    SWEEP_RC=2
else
    SWEEP_OUT=$(python3 "$SIGIL_DRIFT/scripts/drift_paths_sweep.py" \
        --sigil-root "$SIGIL_DRIFT" --ref-tree "$DRIFT_AEON_TREE" \
        --expected-absent "$SIGIL_DRIFT/$DRIFT_EXPECTED_ABSENT" \
        --reads-pattern "$READS_PATTERN" 2>&1)
    SWEEP_RC=$?
fi

# ── observe, then report ─────────────────────────────────────────────────────────
OBSERVE_OUT=$(python3 "$SIGIL_DRIFT/scripts/drift_report.py" observe \
    --ledger "$DRIFT_LEDGER" \
    --aeon-rev "$AEON_SHA" \
    --sigil-linked-rev "$SIGIL_LINKED" \
    --sigil-closure-rev "$SIGIL_CLOSURE" \
    --sigil-tree-state "$SIGIL_TREE" \
    --observed-at "$(date -Is)" \
    --record-reader "$DRIFT_RECORD_READER" \
    "${SHAPE_ARGS[@]}" 2>&1)
OBSERVE_RC=$?

REPORT_OUT=$(python3 "$SIGIL_DRIFT/scripts/drift_report.py" report \
    --ledger "$DRIFT_LEDGER" --n "$N" --n-source "$N_SOURCE" 2>&1)

{
    echo "$(date -Is) run at $AT"
    echo "$OBSERVE_OUT" | sed 's/^/    /'
    echo "$REPORT_OUT" | sed 's/^/    /'
    echo "$SWEEP_OUT" | sed 's/^/    /'
} >> "$LOG"

STATUS=$(sed -n 's/^STATUS: //p' <<< "$REPORT_OUT" | head -1)
SWEEP_LINE=$(head -1 <<< "$SWEEP_OUT")
case "$SWEEP_RC" in
    0) SWEEP_WORD="paths clean" ;;
    1) SWEEP_WORD="PATH DRIFT — $SWEEP_LINE" ;;
    *) SWEEP_WORD="paths NOT SWEPT — $SWEEP_LINE" ;;
esac

# Both halves are named in one line, because a green half must never stand in for the
# other. The accumulated status is carried alongside this run's own so a `NOT A
# VERDICT` reading cannot be lost between the log and the notification.
note "$OBSERVE_OUT | accumulated: ${STATUS:-unreadable} | $SWEEP_WORD | at $AT"
exit "$OBSERVE_RC"
