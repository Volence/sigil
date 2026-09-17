#!/usr/bin/env bash
# The switch-matrix sweep, on a clock.
#
# `scripts/switch_matrix_sweep.py` builds every arm of every build option the Sonic 1
# and Sonic 2 disassemblies declare (the `.asm` ASSEMBLY OPTIONS and build.lua's own
# Settings block), with the stock toolchain through each corpus's build.lua and with
# sigil, and compares the images. It cannot live in the cargo suite: it needs `lua`
# and both corpora. This job is what runs it.
#
# WHAT IT MEASURES, AND AT WHICH REVISIONS. All three trees are named commits, printed
# in the verdict line:
#   sigil     a detached checkout of origin/master (after a fetch), or of
#             SIGIL_SWITCH_SWEEP_REF, built into this job's OWN target directory.
#             Never the shared target/release/sigil, which other lanes relink.
#   corpora   each corpus's committed HEAD (or SIGIL_SWITCH_SWEEP_S1_REF /
#             SIGIL_SWITCH_SWEEP_S2_REF), resolved to a SHA here and handed to the
#             sweep as NAME=PATH@SHA. The sweep extracts that commit with git archive
#             and never reads the working tree, so an uncommitted edit left in a
#             corpus checkout cannot colour the verdict. The number of such entries is
#             printed beside the SHA, so a reader knows the measured tree is not the
#             one on disk.
#
# EXIT CONTRACT, the source-gate lane's:
#   0  the sweep passed and every leg it launched reported
#   1  a FINDING: the sweep ran to completion and failed a reconciliation
#   2  COULD NOT RUN: lua missing, a corpus or ref missing, sigil did not build, the
#      sweep aborted, crashed, timed out or was killed, zero legs, or fewer legs
#      reported than launched or than its own plan lines require
# 1 and 2 notify. A short or reaped run is 2, never 0: the leg reconciliation below is
# read out of the sweep's own per-leg markers, not out of its exit status.
#
# The SIGIL_SWITCH_SWEEP_* variables exist for hand runs and red-first proofs. The
# timer sets none of them.
#
# --selftest-fail exercises the notification path without running anything.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STATE=${SIGIL_SWITCH_SWEEP_STATE:-${XDG_STATE_HOME:-$HOME/.local/state}/sigil-switch-sweep}
LOG="$STATE/nightly.log"
mkdir -p "$STATE"

note() {
    echo "$(date -Is) $1" >> "$LOG"
    echo "$1"
    notify-send -u critical "sigil switch sweep" "$1" 2>/dev/null || true
}

if [[ ${1:-} == --selftest-fail ]]; then
    note "SELFTEST: the failure-notification path works"
    exit 1
fi
if [[ $# -gt 0 ]]; then
    echo "nightly_switch_sweep.sh: refusing unrecognised argument: $1" >&2
    echo "Run bare for the full job (it builds sigil and both corpora many times over)," >&2
    echo "or --selftest-fail to exercise the notification path alone. There is no dry run." >&2
    exit 64
fi

START_EPOCH=$(date +%s)
START_AT=$(date -Is)

# shellcheck source=lib/suite_paths.sh
source "$HERE/lib/suite_paths.sh" \
    || { note "COULD NOT RUN: cannot source $HERE/lib/suite_paths.sh, so no tree can be named"; exit 2; }
SUITE_ROOT=$(suite_resolve_root 2>>"$LOG") \
    || { note "COULD NOT RUN: the suite root could not be resolved, see $LOG"; exit 2; }
SIGIL_MAIN=$(suite_resolve_checkout sigil SIGIL_DIR 2>>"$LOG") \
    || { note "COULD NOT RUN: the sigil checkout could not be resolved, see $LOG"; exit 2; }

# This job's own trees, joined onto the suite root (or a hand run's override). On disk,
# never under /tmp: /tmp is tmpfs here and a cargo build there wedges the shell.
LANE_ROOT=${SIGIL_SWITCH_SWEEP_HOME:-$SUITE_ROOT}
LANE_CHECKOUT="$LANE_ROOT/.sigil-switch-sweep"
export CARGO_TARGET_DIR="$LANE_ROOT/.sigil-switch-sweep-target"
LANE_SCRATCH="$LANE_ROOT/.sigil-switch-sweep-scratch"
SWEEP_LOG="$STATE/sweep.log"

# ── preconditions ────────────────────────────────────────────────────────────────
command -v lua >/dev/null 2>&1 \
    || { note "COULD NOT RUN: no \`lua\` on PATH, and the stock reference build of every leg is each corpus's build.lua"; exit 2; }
command -v python3 >/dev/null 2>&1 \
    || { note "COULD NOT RUN: no \`python3\` on PATH"; exit 2; }

CORPORA=(s1disasm s2disasm)
CORPUS_ARGS=()
CORPUS_AT=""
for c in "${CORPORA[@]}"; do
    case "$c" in
        s1disasm) var=S1DISASM_DIR; ref=${SIGIL_SWITCH_SWEEP_S1_REF:-HEAD} ;;
        s2disasm) var=S2DISASM_DIR; ref=${SIGIL_SWITCH_SWEEP_S2_REF:-HEAD} ;;
    esac
    dir=$(suite_resolve_checkout "$c" "$var" 2>>"$LOG") \
        || { note "COULD NOT RUN: the $c corpus checkout could not be resolved (set $var), see $LOG"; exit 2; }
    sha=$(git -C "$dir" rev-parse --verify "$ref^{commit}" 2>>"$LOG") \
        || { note "COULD NOT RUN: $c ref '$ref' does not name a commit in $dir"; exit 2; }
    dirty=$(git -C "$dir" status --porcelain 2>>"$LOG" | wc -l)
    CORPUS_ARGS+=(--corpus "$c=$dir@$sha")
    CORPUS_AT+=" / $c ${sha:0:8} (working tree: $dirty uncommitted entries, not measured)"
done

# ── sigil, built from a named commit into this job's own target ─────────────────
FETCHED="fetched"
git -C "$SIGIL_MAIN" fetch -q origin >> "$LOG" 2>&1 || FETCHED="NOT fetched, cached tracking ref"
SIGIL_REF=${SIGIL_SWITCH_SWEEP_REF:-origin/master}
SIGIL_SHA=$(git -C "$SIGIL_MAIN" rev-parse --verify "$SIGIL_REF^{commit}" 2>>"$LOG") \
    || { note "COULD NOT RUN: cannot resolve sigil $SIGIL_REF"; exit 2; }
if [[ ! -e "$LANE_CHECKOUT/.git" ]]; then
    git -C "$SIGIL_MAIN" worktree add --detach "$LANE_CHECKOUT" "$SIGIL_SHA" >> "$LOG" 2>&1 \
        || { note "COULD NOT RUN: sigil worktree creation at $LANE_CHECKOUT failed"; exit 2; }
fi
git -C "$LANE_CHECKOUT" checkout --force --detach "$SIGIL_SHA" >> "$LOG" 2>&1 \
    || { note "COULD NOT RUN: sigil checkout of $SIGIL_SHA failed"; exit 2; }
AT="sigil ${SIGIL_SHA:0:8} ($SIGIL_REF, $FETCHED)$CORPUS_AT"

( cd "$LANE_CHECKOUT" && cargo build --release --bin sigil ) > "$STATE/build.log" 2>&1 \
    || { note "COULD NOT RUN: sigil did not build at $AT, see $STATE/build.log"; exit 2; }
SIGIL_BIN="$CARGO_TARGET_DIR/release/sigil"
# The binary is asked which revision it was linked at. A stale binary in this target
# directory would otherwise be measured under the SHA just checked out.
LINKED=$("$SIGIL_BIN" --version 2>&1 | sed -n 's/^  revision:  *\([0-9a-f]\{40\}\)$/\1/p')
[[ $LINKED == "$SIGIL_SHA" ]] \
    || { note "COULD NOT RUN: the built sigil reports revision '${LINKED:-none}', the checkout is $SIGIL_SHA"; exit 2; }

# ── the sweep ────────────────────────────────────────────────────────────────────
# --cross: every corner of the switch space as well as one arm at a time. The corner
# product is the only place the composition prediction and the stock toolchain's own
# refusals are tested, and a nightly slot has the time for it.
rm -rf "$LANE_SCRATCH"
timeout --kill-after=60 4h python3 "$LANE_CHECKOUT/scripts/switch_matrix_sweep.py" \
    --sigil "$SIGIL_BIN" --scratch "$LANE_SCRATCH" --cross "${CORPUS_ARGS[@]}" \
    > "$SWEEP_LOG" 2>&1
rc=$?

END_EPOCH=$(date +%s)
WALL=$(( END_EPOCH - START_EPOCH ))
WALL_TEXT="started $START_AT, $((WALL / 60))m$((WALL % 60))s wall"

case "$rc" in
    0|1) ;;
    124|137) note "COULD NOT RUN: the sweep timed out or was killed (exit $rc) at $AT; $WALL_TEXT; see $SWEEP_LOG"; exit 2 ;;
    *) note "COULD NOT RUN: the sweep aborted with exit $rc at $AT: $(grep -m1 '^SWEEP ABORTED\|^ABORT' "$SWEEP_LOG"); $WALL_TEXT; see $SWEEP_LOG"; exit 2 ;;
esac

# ── reconcile what ran against what was planned ─────────────────────────────────
# Every count is read out of the sweep's log. Each one is checked against another count
# from the same log that was produced by a different line of the sweep, so a run cut
# short, a leg that never reported, or a plan that silently shrank cannot read as green.
count() { grep -c -- "$1" "$SWEEP_LOG"; }
ends=$(count '^SWEEP_END$')
starts=$(count '^   LEG_START ')
reported=$(count '^   LEG_REPORTED ')
errored=$(count '^   LEG_ERROR ')
corpora_seen=$(count '^CORPUS ')
populations=$(count '^POPULATION ')
crosses=$(count '^== CROSS .* -> [0-9]* corners$')
controls=$(count '^   CONTROL C7 [a-z0-9]*: \(PASSED\|FAILED\)')
controls_missing=$(count '^== CONTROL C7 [a-z0-9]*: NOT RUNNABLE')
read -r launched_line reported_line < <(sed -n 's/^RECONCILE legs launched=\([0-9]*\) reported=\([0-9]*\)$/\1 \2/p' "$SWEEP_LOG")
# The plan: a baseline and one leg per arm, each end-to-end control that ran, and every
# corner. Rescue legs are the only ones the plan lines cannot predict, and they are the
# difference printed beside the verdict.
planned=$(awk '
    /^  RECONCILE legs: [0-9]+ arms -> [0-9]+ flip legs \+ 1 baseline$/ { p += $6 + 1 }
    /^== CONTROL C7 [a-z0-9]+: reference built/ { p += 1 }
    /^== CROSS .* -> [0-9]+ corners$/ { p += $(NF - 1) }
    END { print p + 0 }' "$SWEEP_LOG")
options=$(sed -n 's/^POPULATION \([a-z0-9]*\) asm_options=\([0-9]*\) .*build_script_settings=\([0-9]*\) .*corners=\([0-9]*\)$/\1: \2 options, \3 build-script settings, \4 corners/p' "$SWEEP_LOG" | paste -sd';' -)
sigil_line=$(sed -n 's/^SIGIL sigil /sigil /p' "$SWEEP_LOG")

short=""
(( ends == 1 )) || short+=" SWEEP_END seen $ends time(s);"
(( corpora_seen == ${#CORPORA[@]} && populations == ${#CORPORA[@]} )) \
    || short+=" ${#CORPORA[@]} corpora named but $corpora_seen measured ($populations population lines);"
(( crosses == ${#CORPORA[@]} )) || short+=" ${#CORPORA[@]} corpora named but $crosses cross products ran;"
(( controls + controls_missing == ${#CORPORA[@]} )) \
    || short+=" ${#CORPORA[@]} corpora named but $controls end-to-end control verdicts and $controls_missing unrunnable ones;"
[[ -n ${launched_line:-} ]] || short+=" no leg reconciliation line;"
(( starts > 0 )) || short+=" zero legs launched;"
(( ${launched_line:-0} == starts )) || short+=" the sweep counted ${launched_line:-none} launched, its log holds $starts leg starts;"
(( ${reported_line:-0} == starts )) || short+=" the sweep counted ${reported_line:-none} reported against $starts leg starts;"
(( reported + errored == starts )) || short+=" $starts legs started but $reported reported and $errored errored;"
(( planned > 0 && starts >= planned )) || short+=" the plan lines require at least $planned legs, $starts ran;"
[[ $sigil_line == *"(${SIGIL_SHA:0:8})"* ]] || short+=" the sweep reports '$sigil_line', not sigil ${SIGIL_SHA:0:8};"
if [[ $rc == 0 ]]; then
    (( $(count '^SWEEP PASSED$') == 1 )) || short+=" exit 0 without a SWEEP PASSED line;"
else
    (( $(count '^SWEEP FAILED') == 1 )) || short+=" exit 1 without a SWEEP FAILED line;"
fi

SIZE="$starts legs ($planned planned + $((starts - planned)) rescue, $errored errored); $options"
if [[ -n $short ]]; then
    note "COULD NOT RUN: the sweep's run does not reconcile:$short at $AT; $SIZE; $WALL_TEXT; see $SWEEP_LOG"
    exit 2
fi

if (( rc == 0 )); then
    echo "$(date -Is) OK at $AT: $SIZE; $WALL_TEXT" | tee -a "$LOG"
    exit 0
fi

reasons=$(sed -n '/^SWEEP FAILED/,/^SWEEP_END$/p' "$SWEEP_LOG" | grep '^  - ' | cut -c5-200 | paste -sd'|' -)
note "SWEEP FINDING at $AT: $reasons; $SIZE; $WALL_TEXT; see $SWEEP_LOG"
exit 1
