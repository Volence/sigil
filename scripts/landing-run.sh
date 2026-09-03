#!/usr/bin/env bash
# THE LANDING RUN, as one command.
#
# A landing run is the full-suite verification behind a merge to master. It has six
# preconditions, and every one of them is invisible when omitted: the run reads GREEN, or
# it reads RED for a reason that has nothing to do with the code under test. This script
# is the single invocation that carries all six, so the operator remembers one thing
# instead of six.
#
# WHAT THIS DOES NOT DO, stated first because the honest limit is the point.
# A wrapper reduces the omission surface from "remember six things" to "remember one
# thing." IT DOES NOT MAKE OMISSION IMPOSSIBLE, because someone can still not run it.
# Nothing invokes this but a human. There is no timer, no hook, and no gate inside the
# suite that notices a landing run bypassed it. Two follow-ups that WOULD close that,
# neither built here, both named in docs/OVERSEER.md:
#   - a suite-side test comparing each test binary's baked CARGO_MANIFEST_DIR against the
#     tree the run is standing in, which turns the shared-target class from 36 confusing
#     `No such file or directory` reads into one named failure regardless of who ran it;
#   - folding refusals (2) and (4) below into `refreeze --attest`, which already runs the
#     suite and already sets the strict flag, so the freeze path and the landing path
#     would share one set of preconditions.
#
# RELATION TO `refreeze --attest`. That tool covers the same ground for the FREEZE path:
# it sets SIGIL_STRICT_GATE itself, stamps its log up front, and refuses an unmeasurable
# run. It is not a substitute here, because it is bound to the provenance chain — it
# refuses once the tip records a strict run, requires the chain to hold, and APPENDS to
# `provenance.toml`. A merge that moves no ROM bytes has no chain entry to attest, and
# that is the run this script exists for. Where both apply, `--attest` is the one that
# leaves a record; use it, and use this for everything else.
#
# THE SIX, and what each one costs when it is left out:
#
# (1) A DEDICATED, ON-DISK `CARGO_TARGET_DIR`. Never under /tmp: /tmp is tmpfs on this
#     machine and a cargo build there wedges the shell.
#
# (2) A REFUSAL to run against a SHARED `target/`. Cargo bakes the building worktree's
#     `CARGO_MANIFEST_DIR` into the cached rlib, so a target directory that another
#     checkout built into hands this run test binaries that look for their fixtures in
#     THAT tree. Measured twice here: 284 failures once, 36 the next time, both reading
#     exactly like golden divergence, both on files demonstrably present. The log's own
#     stamp truthfully names the correct tree, so refusal (5) cannot catch this one —
#     which is why this is a refusal and not a warning.
#
# (3) `SIGIL_STRICT_GATE=1` INSIDE the command span. Without it every `strict_gate()`
#     guarded port and co-link gate early-returns. A suite that omits it structurally
#     CANNOT EXECUTE the gates a landing exists to move, and reads green.
#
# (4) THE REFERENCE ENVIRONMENT RESOLVED ONCE, refused early and BY NAME, then passed
#     explicitly to the child rather than left to ambient inheritance. A pre-flight that
#     enumerates the state it owns and not the state it depends on dies mid-run: the aeon
#     lane's freeze died inside step 1 on a missing `SIGIL_EMIT`, after the journal was
#     already written.
#
# (5) A STAMPED LOG — pwd, HEAD, branch, the reference tree AND ITS HEAD, and the UTC
#     start. A suite log does not name the tree it measured, and a run from the wrong
#     worktree reads green AND better than the bar.
#
# (6) THE REAL EXIT CODE, in the log and in this script's status. `CARGO_EXIT=` is
#     written from `PIPESTATUS[0]`, not `$?`: with `tee` in the pipeline `$?` is TEE's
#     status, which is the exact trailing-command shape that has misreported a run here
#     — once claiming completion while cargo was still executing, once reporting failure
#     from a trailing `grep`.
#
# Plus the reporting rules a landing verdict is worthless without: failures-first WITH
# THE NAMES (never a tail excerpt, never `grep | head` — that has hidden failures behind
# a merged green here), a `skip:` count, and reconciliation against a baseline the caller
# states. A bare pass count is not a result.
#
# USAGE
#   scripts/landing-run.sh --baseline 4156
#   scripts/landing-run.sh --baseline 4156 --aeon ~/sonic_hacks/.aeon-landing
#   scripts/landing-run.sh --scoped -- -p sigil-span        # a deliberately partial run
#
# WHICH REFERENCE TREE A BARE RUN USES — there is no longer a built-in answer.
#   A run that names no tree does NOT fall back to a live checkout. It resolves one by the
#   suite-paths contract, in this order, and PRINTS which step answered before doing any
#   work, so the log says how the tree was chosen rather than leaving it to be assumed:
#
#     1. `--aeon <path>`, or the AEON_DIR environment variable
#     2. EMPYREAN_SUITE_ROOT joined with `aeon`
#     3. the sibling derived from THIS checkout's own `git rev-parse --git-common-dir`
#     4. otherwise it REFUSES, naming every variable it consulted and every path it tried
#
#   A variable that is set but does NOT name an aeon checkout is a hard error at its own
#   step, not a null that lets the next step run: a wrong value means a wrong environment,
#   and resolving around it would leave that variable wrong for everything downstream.
#   The implementation is scripts/lib/suite_paths.sh, shared with the nightly lanes.
#
#   THE OPT-IN FOR A DELIBERATELY PARTIAL RUN IS `--scoped`, and it is the only one this
#   script has. It does not make a missing reference tree acceptable — it makes the
#   reference-tree ARTIFACTS reported instead of required, and stamps the verdict as
#   PARTIAL so no reader can mistake it for a landing.
#
#   THE SUITE HAS A SECOND, DIFFERENT OPT-IN, AND IT IS NOT THIS SCRIPT'S. Step 4 above is
#   a refusal the TEST SUITE raises (d-18): a bare `cargo test` that resolves no reference
#   tree stops rather than passing green over the reference-dependent rows it silently did
#   not run. The spelling that takes that partial run deliberately is the environment
#   variable `SIGIL_ALLOW_PARTIAL=1`, which leaves every reference-dependent row unmeasured
#   and prints how many binaries that is. It is named here because the refusal a reader
#   meets outside this wrapper names it, and a landing wrapper that never mentioned it
#   would send them looking for a flag this script does not have.
#
#   A LANDING RUN NEVER TAKES IT. `SIGIL_ALLOW_PARTIAL` is REMOVED from the child's
#   environment in the command span below (`env -u`, not an empty assignment: the child
#   then does not carry the variable at all, so this does not depend on how a consumer
#   reads an empty value). An operator who exported it for an earlier bare run cannot
#   carry it into a landing. The two do not compose: this script resolves a tree and
#   passes it explicitly, so the suite's refusal is unreachable here anyway — removing the
#   variable makes that a fact about the environment the child gets rather than a fact
#   about the path taken to build it. `--scoped` is NOT that opt-in and does not set it: a
#   scoped run still requires a real reference tree, and only relaxes which of its built
#   ROMs must be present.
#
# EXIT CODES
#   0  the suite ran, passed, and reconciled against the stated baseline
#   1  the suite FAILED (red tests, or cargo exited nonzero)
#   2  the run COULD NOT RUN or could not be measured — never green, never a count
#   3  the suite passed but the total does NOT reconcile with --baseline

set -uo pipefail

# ---------------------------------------------------------------------------------------
# Refusals speak in one voice, and each names what to do about it.
# ---------------------------------------------------------------------------------------
die() { echo "landing-run: REFUSING — $*" >&2; exit 2; }
say() { echo "landing-run: $*" >&2; }

abspath() { realpath -m -- "$1"; }

# The filesystem type of the nearest EXISTING ancestor, so a target directory that has not
# been created yet is still classified before it is created.
fstype_of() {
    local p; p=$(abspath "$1")
    while [[ ! -e $p && $p != / ]]; do p=$(dirname "$p"); done
    stat -f -c %T -- "$p" 2>/dev/null || echo unknown
}

# ---------------------------------------------------------------------------------------
# Arguments.
# ---------------------------------------------------------------------------------------
BASELINE=""
AEON_ARG=""
TARGET_ARG=""
LOG_ARG=""
SCOPED=0
CARGO_EXTRA=()
EXPECT=()

while (( $# )); do
    case $1 in
        --baseline) BASELINE=${2:-}; shift 2 || die "--baseline needs a number" ;;
        --aeon)     AEON_ARG=${2:-}; shift 2 || die "--aeon needs a path" ;;
        --target)   TARGET_ARG=${2:-}; shift 2 || die "--target needs a path" ;;
        --log)      LOG_ARG=${2:-}; shift 2 || die "--log needs a path" ;;
        # Repeatable. A green log that does not contain the landed code's own test is a
        # green log about other code.
        --expect-test) EXPECT+=("${2:-}"); shift 2 || die "--expect-test needs a name" ;;
        # Says OUT LOUD that this run is deliberately partial. Without it an unscoped run
        # is assumed and the reference-tree artifacts are required; with it they are
        # reported rather than required, and the verdict says the run was partial so no
        # reader can mistake it for a landing.
        --scoped)   SCOPED=1; shift ;;
        --)         shift; CARGO_EXTRA=("$@"); break ;;
        # The header, to wherever it actually ends. This was `sed -n '2,80p'`, and a hard
        # line number is a help text that silently truncates the moment the header grows —
        # which it just did. The end of the header is a fact about the file, so it is read
        # from the file.
        -h|--help)  awk '/^set -uo pipefail/ { exit } NR > 1' "$0"; exit 0 ;;
        *)          die "unknown argument \`$1\` (try --help)" ;;
    esac
done

if (( ${#CARGO_EXTRA[@]} )) && (( ! SCOPED )); then
    die "extra cargo arguments were given (${CARGO_EXTRA[*]}) but --scoped was not.
       Passing a filter makes the run PARTIAL, and a partial run recorded as a landing is
       the failure this script exists to prevent. Add --scoped to say so on purpose."
fi

# ---------------------------------------------------------------------------------------
# (0) Where we are. Everything below is derived from this, never from the caller's cwd.
# ---------------------------------------------------------------------------------------
ROOT=$(git rev-parse --show-toplevel 2>/dev/null) \
    || die "not inside a git checkout — this must run from a sigil worktree"
ROOT=$(abspath "$ROOT")
[[ -f $ROOT/Cargo.toml ]] || die "$ROOT has no Cargo.toml — that is not the sigil workspace"

# The MAIN checkout, which for a linked worktree is the parent of the common git dir. Its
# `target/` is the shared one, and it is the directory refusal (2) is really about: this
# worktree's own `target/` is merely the other name for the same mistake.
COMMON=$(git rev-parse --git-common-dir 2>/dev/null) || die "cannot resolve the git common dir"
MAIN=$(abspath "$(dirname "$(abspath "$COMMON")")")

HEAD_SHA=$(git -C "$ROOT" rev-parse HEAD 2>/dev/null) || die "cannot resolve HEAD"
BRANCH=$(git -C "$ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')
DIRTY=clean
[[ -n $(git -C "$ROOT" status --porcelain 2>/dev/null) ]] && DIRTY=DIRTY

# ---------------------------------------------------------------------------------------
# (1)+(2) The target directory, and the refusal that is the whole point.
# ---------------------------------------------------------------------------------------
TARGET=$(abspath "${TARGET_ARG:-${SIGIL_LANDING_TARGET:-$ROOT/.target-land}}")

# The two spellings of the shared directory. Both are refused, and so is any `target/`
# belonging to a checkout that is not this one.
for forbidden in "$ROOT/target" "$MAIN/target"; do
    if [[ $TARGET == "$(abspath "$forbidden")" ]]; then
        die "the target directory is $TARGET, which is a checkout's DEFAULT \`target/\`.
       Cargo bakes the building worktree's path into its cached artifacts, so a target
       directory shared with another checkout hands this run test binaries that look for
       their fixtures in a DIFFERENT tree. That surfaces as dozens of
       \`read <file>: No such file or directory\` failures on files that are present, and
       it reads exactly like golden divergence.
       Use a dedicated directory — the default \`$ROOT/.target-land\` is gitignored — or
       pass --target <dir>."
    fi
done

case "$TARGET" in
    /tmp|/tmp/*) die "the target directory is $TARGET. /tmp is tmpfs on this machine and a
       cargo build there wedges the shell. Put it on disk." ;;
esac
FS=$(fstype_of "$TARGET")
if [[ $FS == tmpfs || $FS == ramfs ]]; then
    die "the target directory $TARGET is on a $FS (RAM-backed) filesystem. A cargo build
       there wedges the shell. Put it on disk."
fi

mkdir -p "$TARGET" || die "cannot create the target directory $TARGET"

# THE OWNERSHIP MARKER. Refusing the two DEFAULT `target/` paths only covers the mistake
# people make by not setting the variable; it says nothing about two worktrees pointed at
# one dedicated directory, which is the same poisoning by a different route. The marker
# records which tree built into this directory, so the second tree is refused BY NAME
# rather than discovered as a wall of missing-file failures.
OWNER_FILE="$TARGET/.sigil-landing-owner"
if [[ -f $OWNER_FILE ]]; then
    OWNER=$(cat "$OWNER_FILE" 2>/dev/null || true)
    if [[ -n $OWNER && $OWNER != "$ROOT" ]]; then
        die "the target directory $TARGET was last built into by a DIFFERENT checkout:
         owner: $OWNER
         this:  $ROOT
       Cargo would reuse artifacts baked with the owner's paths. Give this tree its own
       directory (--target), or delete $TARGET if the owner is gone."
    fi
fi
printf '%s\n' "$ROOT" > "$OWNER_FILE" || die "cannot write the ownership marker $OWNER_FILE"

# ---------------------------------------------------------------------------------------
# (4) The reference environment: resolved ONCE, refused BY NAME, passed EXPLICITLY.
# ---------------------------------------------------------------------------------------
# `--aeon <path>` is STEP 1 by another spelling — the operator naming the tree on this
# command line is at least as explicit as the environment doing it, and it is checked
# here rather than handed to the include so the refusal can name the flag.
#
# Absent the flag, the include implements the whole precedence and ANNOUNCES which step
# answered. There is no home literal left in this line: the predecessor's default sent a
# run that named no tree at the owner's live checkout, which is mid-edit, carries his
# content edits, and is not at the provenance tip — a green from it is a green about
# something nobody chose.
# shellcheck source=lib/suite_paths.sh
source "$ROOT/scripts/lib/suite_paths.sh" \
    || die "cannot source $ROOT/scripts/lib/suite_paths.sh — the reference tree cannot be
       resolved without it, and guessing is what this script exists to stop."

if [[ -n $AEON_ARG ]]; then
    AEON=$(abspath "$AEON_ARG")
    AEON_STEP="1: explicit --aeon"
    suite_paths_announce AEON_DIR "$AEON" 1 "explicit --aeon"
else
    # The include's own announce goes to stderr; its refusal names every variable
    # consulted and every path tried, which is exactly refusal (4)'s bar, so it is let
    # through verbatim rather than re-worded into something less specific.
    AEON=$(suite_resolve_checkout aeon AEON_DIR) \
        || die "the reference tree could not be resolved (see the refusal above).
       Pass --aeon <path to a built aeon checkout>, or export AEON_DIR."
    AEON=$(abspath "$AEON")
    # Which step answered is on stderr from the include and repeated into the log stamp
    # below, so the run's own record says how the tree was named. Re-derived rather than
    # captured because the announce is stderr and the path is stdout; the two spellings
    # agree by construction (both come from the same variables the include read).
    if [[ -n ${AEON_DIR:-} ]]; then AEON_STEP="1: explicit AEON_DIR"
    elif [[ -n ${EMPYREAN_SUITE_ROOT:-} ]]; then AEON_STEP="2: EMPYREAN_SUITE_ROOT/aeon"
    else AEON_STEP="3: sibling of this checkout via git --git-common-dir"
    fi
fi
[[ -d $AEON ]] || die "AEON_DIR resolves to $AEON, which is not a directory.
       Pass --aeon <path to a built aeon checkout>, or export AEON_DIR."
[[ -f $AEON/build.sh ]] || die "AEON_DIR resolves to $AEON, which has no build.sh — that is
       not an aeon checkout. Pass --aeon <path to a built aeon checkout>."

AEON_HEAD=$(git -C "$AEON" rev-parse HEAD 2>/dev/null || echo '?')
AEON_BRANCH=$(git -C "$AEON" rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')
AEON_DIRTY=clean
[[ -n $(git -C "$AEON" status --porcelain 2>/dev/null) ]] && AEON_DIRTY=DIRTY

# SIGIL_BUILD and SIGIL_EMIT default INTO this run's own target directory and are built
# there if absent, so the binaries a landing uses are the ones this tree just compiled.
# An override is honoured and stamped, because a caller pinning a specific assembler is a
# real case — but the stamp is what makes the log answerable about which binary ran.
SIGIL_BUILD_RESOLVED=${SIGIL_BUILD:-$TARGET/release/sigil}
SIGIL_EMIT_RESOLVED=${SIGIL_EMIT:-$TARGET/release/emit_sound_blob}
BUILD_ORIGIN=derived; [[ -n ${SIGIL_BUILD:-} ]] && BUILD_ORIGIN=overridden
EMIT_ORIGIN=derived;  [[ -n ${SIGIL_EMIT:-} ]] && EMIT_ORIGIN=overridden

# AN OVERRIDE IS CHECKED FIRST, BEFORE ANY BUILD. The caller named this path, so nothing
# this script does can make it appear, and spending a compile before saying so is the
# "refuse early" rule failing on its own terms.
if [[ $BUILD_ORIGIN == overridden && ! -x $SIGIL_BUILD_RESOLVED ]]; then
    die "SIGIL_BUILD is set to $SIGIL_BUILD_RESOLVED, which is not an executable file.
       Unset it to use this run's own build, or point it at a real binary."
fi
if [[ $EMIT_ORIGIN == overridden && ! -x $SIGIL_EMIT_RESOLVED ]]; then
    die "SIGIL_EMIT is set to $SIGIL_EMIT_RESOLVED, which is not an executable file.
       Unset it to use this run's own build, or point it at a real binary."
fi

# A DERIVED path is this script's to produce, so it is built rather than demanded: these
# two binaries live in the workspace the suite is about to compile anyway, and building
# them HERE is what guarantees the landing uses the assembler this tree just made rather
# than whatever a shared directory last left lying around.
if [[ ! -x $SIGIL_BUILD_RESOLVED || ! -x $SIGIL_EMIT_RESOLVED ]]; then
    say "building sigil + emit_sound_blob into $TARGET (first run in this directory)"
    CARGO_TARGET_DIR="$TARGET" cargo build --release --manifest-path "$ROOT/Cargo.toml" \
        --bin sigil --bin emit_sound_blob >&2 \
        || die "the pre-flight build of \`sigil\` and \`emit_sound_blob\` failed. Nothing ran."
fi

# BY NAME, one at a time. Reached only for derived paths, and only if the build above
# claimed success without producing them.
[[ -x $SIGIL_BUILD_RESOLVED ]] \
    || die "SIGIL_BUILD is $SIGIL_BUILD_RESOLVED ($BUILD_ORIGIN), which is not an executable
       file — the pre-flight build reported success without producing it."
[[ -x $SIGIL_EMIT_RESOLVED ]] \
    || die "SIGIL_EMIT is $SIGIL_EMIT_RESOLVED ($EMIT_ORIGIN), which is not an executable
       file — the pre-flight build reported success without producing it."

# THE CHECK THAT ACTUALLY PREDICTS THE ARTIFACT GATES. The suite does not read SIGIL_EMIT
# or SIGIL_BUILD (it emits the sound blob in-process); what the ~80 port and golden gates
# read is the BUILT ROMs in the reference tree. A tree that has never been built fails
# them by the dozen, and `build.sh` makes one shape per invocation, so a half-built tree
# is the common shape. Required for a full landing; reported for a --scoped run, because
# refusing a partial run that reads none of them would refuse a correct case.
MISSING_ROMS=()
for rom in s4.bin s4.debug.bin demo.bin demo.debug.bin; do
    [[ -f $AEON/$rom ]] || MISSING_ROMS+=("$rom")
done
ROM_STATE="all four present"
if (( ${#MISSING_ROMS[@]} )); then
    ROM_STATE="MISSING: ${MISSING_ROMS[*]}"
    if (( SCOPED )); then
        say "WARNING — the reference tree $AEON is missing ${#MISSING_ROMS[@]} built ROM(s):
       ${MISSING_ROMS[*]}. This is a --scoped run so it is not refused, but every
       artifact-dependent gate that runs will be red for this reason and not for yours."
    else
        die "the reference tree $AEON is missing ${#MISSING_ROMS[@]} of the four built ROMs:
       ${MISSING_ROMS[*]}
       The port and golden gates read these directly; without them a full run is red for a
       provisioning reason that looks exactly like a regression. Build all four shapes
       there (build.sh emits ONE per invocation: plain and DEBUG=1, for sonic4 and demo)
       with SIGIL_EMIT=$SIGIL_EMIT_RESOLVED, or pass --scoped to say this run is partial."
    fi
fi

# ---------------------------------------------------------------------------------------
# (5) The stamp, written BEFORE cargo writes a byte.
# ---------------------------------------------------------------------------------------
STARTED=$(date -u +%Y-%m-%dT%H:%M:%SZ)
LOG=${LOG_ARG:-$TARGET/landing-$(date -u +%Y%m%dT%H%M%SZ).log}
mkdir -p "$(dirname "$LOG")" || die "cannot create the log directory for $LOG"

# `--manifest-path` goes BEFORE the `--`. Everything after `--` belongs to the test
# harness, so a cargo flag placed there is silently handed to libtest instead.
CARGO_ARGS=(test --release --no-fail-fast --manifest-path "$ROOT/Cargo.toml")
# A landing is the WHOLE workspace. A --scoped run drops `--workspace` so the caller's
# filter is the selection rather than fighting one, which is also why --scoped has to be
# said out loud: the two runs are not the same object and only one of them is a landing.
if (( SCOPED )); then
    CARGO_ARGS+=("${CARGO_EXTRA[@]}")
else
    CARGO_ARGS+=(--workspace)
fi
CARGO_ARGS+=(-- --nocapture)

{
    echo "# sigil landing run"
    echo "# started (UTC)  $STARTED"
    echo "# pwd            $ROOT"
    echo "# sigil HEAD     $HEAD_SHA"
    echo "# sigil branch   $BRANCH ($DIRTY)"
    echo "# main checkout  $MAIN"
    echo "# AEON_DIR       $AEON (step $AEON_STEP)"
    echo "# aeon HEAD      $AEON_HEAD"
    echo "# aeon branch    $AEON_BRANCH ($AEON_DIRTY)"
    echo "# aeon ROMs      $ROM_STATE"
    echo "# TARGET_DIR     $TARGET"
    echo "# SIGIL_BUILD    $SIGIL_BUILD_RESOLVED ($BUILD_ORIGIN)"
    echo "# SIGIL_EMIT     $SIGIL_EMIT_RESOLVED ($EMIT_ORIGIN)"
    echo "# scoped         $( ((SCOPED)) && echo 'YES — this is a PARTIAL run, not a landing' || echo 'no (full workspace)')"
    echo "# baseline       ${BASELINE:-<none stated>}"
    # The suite's own partial-run opt-in, stamped as CLEARED rather than left to be
    # assumed: the log has to be answerable about whether the reference-dependent rows
    # were measured, and "the variable was not set in my shell" is not something a
    # later reader of this file can check.
    echo "# allow-partial  removed from the child (was: ${SIGIL_ALLOW_PARTIAL:-<unset>})"
    echo "# command        SIGIL_STRICT_GATE=1 env -u SIGIL_ALLOW_PARTIAL cargo ${CARGO_ARGS[*]}"
    echo
} > "$LOG" || die "cannot stamp the log $LOG"

say "log -> $LOG"
say "(tail it: tail -f $LOG)"

# ---------------------------------------------------------------------------------------
# (3)+(6) The run. The strict flag is INSIDE the command span; the exit code is cargo's.
# ---------------------------------------------------------------------------------------
SIGIL_STRICT_GATE=1 \
CARGO_TARGET_DIR="$TARGET" \
AEON_DIR="$AEON" \
SIGIL_BUILD="$SIGIL_BUILD_RESOLVED" \
SIGIL_EMIT="$SIGIL_EMIT_RESOLVED" \
    env -u SIGIL_ALLOW_PARTIAL cargo "${CARGO_ARGS[@]}" 2>&1 | tee -a "$LOG"
# PIPESTATUS[0], never `$?`. With `tee` in the pipeline `$?` is tee's status, and a
# wrapper reporting a trailing command's code as the run's verdict is the exact defect
# this line exists to close.
CARGO_RC=${PIPESTATUS[0]}
echo "CARGO_EXIT=$CARGO_RC" >> "$LOG"

FINISHED=$(date -u +%Y-%m-%dT%H:%M:%SZ)
echo "# finished (UTC) $FINISHED" >> "$LOG"

# ---------------------------------------------------------------------------------------
# (7) Failures first, WITH the names. No `head`, no tail excerpt.
# ---------------------------------------------------------------------------------------
# `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; …`
# The label token carries a trailing `;`, so the comparison strips it. Matching on the
# bare word silently sums nothing and reports a real run as zero.
read -r SUITES PASSED FAILED IGNORED < <(awk '
    /^test result:/ {
        n++
        for (i = 1; i < NF; i++) {
            lbl = $(i+1); sub(/;$/, "", lbl)
            if ($i ~ /^[0-9]+$/) {
                if (lbl == "passed")  p += $i
                if (lbl == "failed")  f += $i
                if (lbl == "ignored") g += $i
            }
        }
    }
    END { print n+0, p+0, f+0, g+0 }' "$LOG")
# BOTH spellings. The landing bar greps `skip:`, and 27 sites say `skipping` instead —
# invisible to that grep while reporting green. A matcher inheriting the same blind spot
# would under-count while still looking like a witness.
SKIPS=$(grep -cE 'skip:|skipping' "$LOG" || true)

# Every failing name, sorted and deduped. All of them.
mapfile -t FAILING < <(grep -E '^test .* \.\.\. FAILED$' "$LOG" \
    | sed -E 's/^test (.*) \.\.\. FAILED$/\1/' | sort -u)

echo
echo "=============================== LANDING RUN VERDICT ==============================="
echo "  log             $LOG"
echo "  tree            $ROOT @ ${HEAD_SHA:0:8} ($BRANCH, $DIRTY)"
echo "  reference       $AEON @ ${AEON_HEAD:0:8} ($AEON_BRANCH, $AEON_DIRTY) — $ROM_STATE"
echo "  target dir      $TARGET"
echo "  started/ended   $STARTED -> $FINISHED (UTC)"
echo "  CARGO_EXIT      $CARGO_RC"
(( SCOPED )) && echo "  ** SCOPED RUN — PARTIAL. This is not a landing verdict. **"

# UNMEASURABLE IS NOT GREEN. Both branches below are runs whose result cannot be
# classified, and neither may be rendered as a count.
if (( SUITES == 0 )); then
    echo "  RESULT          COULD NOT RUN — no \`test result:\` line in the log."
    echo "                  Nothing about this run can be measured. cargo exited $CARGO_RC."
    echo "==================================================================================="
    exit 2
fi
if (( PASSED == 0 )); then
    echo "  RESULT          COULD NOT RUN — $SUITES suite(s) reported, 0 tests passed."
    echo "                  That is a run that could not be measured, not a green one."
    echo "==================================================================================="
    exit 2
fi

echo "  suites          $SUITES"
echo "  passed          $PASSED"
echo "  failed          $FAILED"
echo "  ignored         $IGNORED"
if (( SKIPS > 0 )); then
    echo "  skip lines      $SKIPS   <-- WARNING: SIGIL_STRICT_GATE=1 should make these"
    echo "                          impossible. Each is a gate that measured nothing while"
    echo "                          reporting green, so this run's green means less than it"
    echo "                          reads. Grep the log for 'skip:' and 'skipping'."
else
    echo "  skip lines      0"
fi

if (( ${#FAILING[@]} )); then
    echo
    echo "  FAILING TESTS (${#FAILING[@]}), all of them:"
    for t in "${FAILING[@]}"; do echo "    $t"; done
fi

# Named expectations: a green log lacking the landed code's own test is about other code.
MISSING_EXPECT=()
for name in "${EXPECT[@]:-}"; do
    [[ -z $name ]] && continue
    # Literal, not a regex: a test name carrying `::` or `[` must match as text.
    awk -v n="$name" 'index($0,"test ")==1 && index($0,n) && index($0," ... ") {f=1}
                      END {exit !f}' "$LOG" || MISSING_EXPECT+=("$name")
done
if (( ${#MISSING_EXPECT[@]} )); then
    echo
    echo "  --expect-test NAME(S) THAT DID NOT EXECUTE: ${MISSING_EXPECT[*]}"
    echo "  A green log that does not contain the landed code's own test is a green log"
    echo "  about other code."
    echo "==================================================================================="
    exit 2
fi

# ---------------------------------------------------------------------------------------
# (8) Reconciliation. A bare pass count is not a result.
# ---------------------------------------------------------------------------------------
RECONCILED=1
if [[ -n $BASELINE ]]; then
    if [[ ! $BASELINE =~ ^[0-9]+$ ]]; then
        echo "  baseline        INVALID (\`$BASELINE\` is not a number) — nothing reconciled."
        RECONCILED=0
    else
        # Reconcile on tests that RETURNED A VERDICT — passed plus failed — never on
        # PASSED alone. `--baseline` is the count expected to pass on a GREEN run, so on
        # a red run every failure is a test missing from PASSED for a reason the operator
        # can already see in the failing list. Comparing PASSED alone charges those to
        # the did-not-run column and then says so in words: the old message asserted
        # "Tests did not fail — they did not run" on a run whose verdict block, eight
        # lines above, was listing the tests that failed. The count and the sentence
        # disagreed with each other and the sentence is the half a reader carries away.
        # With zero failures this is arithmetically identical to the old form, so the
        # bar has not moved — it stopped mis-attributing a red run's shortfall.
        RAN=$(( PASSED + FAILED ))
        DELTA=$(( RAN - BASELINE ))
        if (( DELTA >= 0 )); then
            if (( FAILED > 0 )); then
                echo "  reconciles      $BASELINE baseline + $DELTA new = $RAN returned a verdict ($PASSED passed + $FAILED failed)"
            else
                echo "  reconciles      $BASELINE baseline + $DELTA new = $PASSED observed"
            fi
        else
            echo "  reconciles      MISMATCH: baseline $BASELINE, observed $RAN returning a verdict"
            echo "                  ($PASSED passed + $FAILED failed). ${DELTA#-} test(s) FEWER than the"
            echo "                  stated baseline, and the failures above do NOT account for"
            echo "                  them — these did not run at all. Something stopped being"
            echo "                  built, was filtered out, or was marked #[ignore]."
            RECONCILED=0
        fi
    fi
else
    echo "  reconciles      NOT CHECKED — no --baseline stated. A bare pass count is not a"
    echo "                  result; state the baseline you expect."
fi

# ---------------------------------------------------------------------------------------
# The verdict. Derived from cargo's own status and the measurement — never from whatever
# command happened to run last.
# ---------------------------------------------------------------------------------------
if (( CARGO_RC != 0 || FAILED > 0 )); then
    echo
    echo "  RESULT          FAILED — $FAILED test(s) red, cargo exit $CARGO_RC."
    echo "==================================================================================="
    exit 1
fi
if (( ! RECONCILED )); then
    echo
    echo "  RESULT          GREEN BUT UNRECONCILED — every test that ran passed, and the"
    echo "                  population is not the one you stated. Do not land on this."
    echo "==================================================================================="
    exit 3
fi
echo
echo "  RESULT          GREEN"
echo "==================================================================================="
exit 0
