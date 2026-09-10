#!/usr/bin/env bash
# THE LANDING RUN, as one command.
#
# A landing run is the full-suite verification behind a merge to master. It has seven
# preconditions, and every one of them is invisible when omitted: the run reads GREEN, or
# it reads RED for a reason that has nothing to do with the code under test. This script
# is the single invocation that carries all seven, so the operator remembers one thing
# instead of seven.
#
# WHAT THIS DOES NOT DO, stated first because the honest limit is the point.
# A wrapper reduces the omission surface from "remember seven things" to "remember one
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
# THE SEVEN, and what each one costs when it is left out:
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
# (7) THE LINT BAR, RUN HERE RATHER THAN BY HAND. The stated bar is
#     `cargo clippy --workspace --all-targets -- -D warnings` exiting 0 (README.md). Until
#     this script ran it, it did not: the wrapper printed `RESULT GREEN` having never
#     invoked clippy, so the only thing standing between a red lint bar and a merge was an
#     operator remembering a second command — which is the omission surface (1)-(6) exist
#     to remove, left open for the seventh.
#
#     WHAT THE MISSING FLAG ACTUALLY IS. The hand-run form people reach for is bare
#     `cargo clippy`, and its clean exit proves only that nothing was an ERROR — clippy's
#     own lints are warnings, so it prints every finding and still exits 0. Measured on
#     this tree at the branch point of the commit that added this section, where 116
#     `tabs_in_doc_comments` sites stood across seven `sigil-frontend-as` files:
#
#       cargo clippy                                          -> exit 0, findings PRINTED
#       cargo clippy --release --workspace --all-targets       -> exit 0, findings PRINTED
#       cargo clippy --release --workspace --all-targets -Dwarn -> exit 101
#
#     So `-D warnings` is the flag that turns clippy into an instrument with a verdict;
#     without it the command is a reporter, and a reporter's exit code answers a question
#     nobody asked. `--all-targets` is the second half and is equally load-bearing: 106 of
#     those 116 sites live in TEST targets, so with the lib alone fixed the shorter
#     `--release --workspace -- -D warnings` exits 0 while `--all-targets` still exits 101.
#     A lint bar that does not read the test and bench targets was blind to 91% of what
#     was red.
#
#     A COUNT TAKEN UNDER `-D warnings` IS NOT THE POPULATION. It is how far the build
#     got: the first crate to fail aborts, cargo stops scheduling the rest, and the
#     remaining targets are never checked. The same tree reported 10, then 35 more, then
#     71 more, as each round of fixes let the next target compile. To size the work, run
#     clippy WITHOUT `-D warnings` — nothing aborts, everything is checked — and count
#     from that. This script does not need the distinction (it wants a verdict, and any
#     nonzero exit is one), but anyone reading its lint list to plan a fix does.
#
#     A RED LINT BAR MAKES THIS SCRIPT'S RESULT NOT-GREEN, exactly as a red test does, and
#     `CLIPPY_EXIT` sits beside `CARGO_EXIT` in the verdict block. There is no --no-clippy
#     and no skip path, including for `--scoped`: a scoped run already says out loud that
#     it is not a landing, and adding a second way to get a green-looking verdict without
#     the lint bar would rebuild the hole this closes. THE FIX FOR A RED BAR IS NEVER A
#     WORKSPACE-WIDE OR CRATE-WIDE `allow` — that turns a correct lint off everywhere to
#     settle one site. Silence the specific item, with a comment saying why, or change the
#     code.
#
# (8) THE LEDGER GATE, `scripts/ledger_gate.py`, RUN HERE FOR THE SAME REASON AS (7).
#     `tools/decisions_reader_audit.py` was committed, correct, and had NO CALLER
#     ANYWHERE IN THE TREE. It answers "which lines of docs/decisions.jsonl can the
#     owner's console actually render", and between 2026-08-30 and 2026-09-09 that answer
#     went from 3 of 16 to 12 of 36 -- the file doubled and the unrenderable share went
#     19% to 33% with nobody noticing, because nothing ran it. AN UNWIRED AUDIT DOES NOT
#     STOP THE NUMBER GROWING, IT STOPS ANYONE NOTICING THAT IT GREW. Same class as (7):
#     a bar that exists and that only an operator remembering a second command stands
#     between and a merge.
#
#     THREE ASSERTIONS AT THREE STRENGTHS, stated in full in that script's header. JSON
#     well-formedness of every `docs/*.jsonl` is HARD; console-renderability is a RATCHET
#     that fails on GROWTH past a measured pin and on nothing else, because a hard gate
#     there is red on arrival against ratified history and the remedy a reasonable person
#     reaches for is weakening it; id-uniqueness is REPORTED AND IS NOT A GATE, because
#     wiring it before the rule-8e re-id is made would hand this repo an unlandable
#     master.
#
#     A RED LEDGER GATE MAKES THIS SCRIPT'S RESULT NOT-GREEN, and `LEDGER_EXIT` sits
#     beside `CARGO_EXIT` and `CLIPPY_EXIT` in the verdict block. There is no --no-ledger
#     and no skip path, INCLUDING FOR `--scoped`: the gate reads files in this checkout
#     and needs no reference tree, so a scoped run has nothing to be partial about here,
#     and a second way to reach a green-looking verdict without it would rebuild the hole
#     this closes.
#
#     WHERE ITS TWO HALVES ARE. The measurement is in the block marked `(9) THE LEDGER
#     GATE` below; the decision is `LEDGER_RC != 0` in the `RESULT FAILED` condition at
#     the foot of this file. THEY ARE HUNDREDS OF LINES APART, which is the shape that
#     makes a correct collect-then-decide gate read as decorative to anyone who stops at
#     the first half -- oracle's tools/land.sh has the same split, its `fail()` at :260
#     merely appends to an array and its `finish_red` at :663 is what exits 1. So each
#     half here names the other, and the wiring is proven by a control that makes the gate
#     red on purpose and observes this script REFUSE, not by reading either half:
#     `crates/sigil-harness/tests/landing_verdict.rs`.
#
# Plus the reporting rules a landing verdict is worthless without: failures-first WITH
# THE NAMES (never a tail excerpt, never `grep | head` — that has hidden failures behind
# a merged green here), a `skip:` count THAT FAILS THE RUN WHEN IT IS NOT ZERO, and
# reconciliation against a baseline the caller states. A bare pass count is not a result.
#
#     A SKIP LINE IS A RED RUN. Under `SIGIL_STRICT_GATE=1` every reference-dependent
#     gate must either measure or panic, so a `skip:` (or `skipping`) line inside the test
#     span is a gate that measured nothing while reporting green. It is counted in the
#     verdict block and it sits in the SAME condition as a red test and a red lint bar:
#     `RESULT FAILED`, exit 1. It was a warning line beside `RESULT GREEN` until this was
#     written, which is the exact shape (7) closed for clippy: a bar reported beside a
#     green verdict is a bar that gets landed over.
#
#     A BINARY THAT LAUNCHED AND NEVER REPORTED IS A RED RUN. The totals are sums over
#     `test result:` lines, and until this was written NOTHING SAID WHAT THAT COUNT SHOULD
#     BE. A test binary that dies mid-run prints no `test result:` line, so its tests leave
#     `suites` and `passed` silently and every line in the verdict still reads like a
#     complete run. `--baseline` does not cover it: the reconciliation fails only on a
#     shortfall, so it is a lower bound, and every test added since the caller last moved
#     that number is slack a vanished test hides inside (measured here: 0 to 21 tests
#     across seventeen landing logs). The check pairs cargo's own `Running`/`Doc-tests`
#     launch lines against the children's `test result:` lines and NAMES any binary that
#     started and went quiet.
#
#     A TEST-TARGET CENSUS SHORTFALL IS A RED RUN, and it is the ONE population in the
#     verdict that does not come out of the log. The pairing above is structurally blind to
#     a target that stopped being BUILT: such a target neither launches nor reports, so it
#     is absent from both of the log's own populations and a check comparing them would be
#     asserting a set against itself. `scripts/test_target_census.py` reads the workspace
#     manifests through `cargo metadata --no-deps`, which builds nothing, and the figure is
#     STAMPED into the log so `--verdict-only` judges the tree the run was made from. A
#     census that could not be taken fails too; a log written before the stamp existed says
#     NOT STATED and does not.
#
# USAGE
#   scripts/landing-run.sh --baseline 4156
#   scripts/landing-run.sh --baseline 4156 --aeon ~/sonic_hacks/.aeon-landing
#   scripts/landing-run.sh --scoped -- -p sigil-span        # a deliberately partial run
#   scripts/landing-run.sh --verdict-only <log>             # re-judge a finished log
#
#   `--verdict-only <log>` runs NOTHING: it reads a log this script (or a fixture shaped
#   like one) already wrote and puts it through the identical verdict code path, so the
#   verdict rules can be exercised and tested without a suite run. The stamp lines,
#   `CARGO_EXIT=`, `CLIPPY_EXIT=` and `LEDGER_EXIT=` are read out of the log; a log
#   carrying no exit lines is refused, because a verdict over an unfinished run is not a
#   verdict.
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
#   0  the suite ran, passed, reconciled against the stated baseline, and the lint bar
#      exited 0
#   1  the suite FAILED (red tests, cargo exited nonzero, or a `skip:` line survived
#      SIGIL_STRICT_GATE=1), or THE LINT BAR IS RED, or THE LEDGER GATE IS RED
#   2  the run COULD NOT RUN or could not be measured — never green, never a count
#   3  the suite passed but the total does NOT reconcile with --baseline

set -uo pipefail

# ---------------------------------------------------------------------------------------
# Refusals speak in one voice, and each names what to do about it.
# ---------------------------------------------------------------------------------------
die() { echo "landing-run: REFUSING, $*" >&2; exit 2; }
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
VERDICT_ONLY=""
CARGO_EXTRA=()
EXPECT=()

while (( $# )); do
    case $1 in
        --baseline) BASELINE=${2:-}; shift 2 || die "--baseline needs a number" ;;
        --aeon)     AEON_ARG=${2:-}; shift 2 || die "--aeon needs a path" ;;
        --target)   TARGET_ARG=${2:-}; shift 2 || die "--target needs a path" ;;
        --log)      LOG_ARG=${2:-}; shift 2 || die "--log needs a path" ;;
        # Runs nothing. The verdict rules below are read out of an EXISTING log, so they
        # can be exercised (and red-first tested) without a suite run.
        --verdict-only) VERDICT_ONLY=${2:-}; shift 2 || die "--verdict-only needs a log path" ;;
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
# --verdict-only: the verdict's inputs, read out of a finished log instead of produced by
# a run. Every value the verdict block prints or tests is set here from the stamp this
# script writes, so the SAME verdict code runs over a log whether the run happened in
# this process or in one that finished last week. Nothing below writes to the log.
# ---------------------------------------------------------------------------------------
# The stamp line for a key, with the key and its padding stripped. Empty when absent.
stamp() { sed -n "s/^# $1 *//p" "$LOG" | head -n 1; }

load_verdict_inputs() {
    LOG=$(abspath "$1")
    [[ -f $LOG ]] || die "--verdict-only: $LOG is not a readable log file"
    # The two exit lines are the run's own verdict inputs, and a log without them is a
    # run that did not finish (or a fixture that forgot them). Refused BY NAME rather than
    # defaulted to 0, because a defaulted exit code is a green that nobody measured.
    CARGO_RC=$(sed -n 's/^CARGO_EXIT=//p' "$LOG" | tail -n 1)
    CLIPPY_RC=$(sed -n 's/^CLIPPY_EXIT=//p' "$LOG" | tail -n 1)
    [[ $CARGO_RC =~ ^[0-9]+$ ]] \
        || die "--verdict-only: $LOG carries no \`CARGO_EXIT=<n>\` line, the test span never
       finished, so there is no verdict to give over it."
    [[ $CLIPPY_RC =~ ^[0-9]+$ ]] \
        || die "--verdict-only: $LOG carries no \`CLIPPY_EXIT=<n>\` line, the lint bar was never
       measured, so there is no verdict to give over it."
    # REFUSED BY NAME, NOT DEFAULTED TO 0, for the identical reason as the two above. A
    # ledger gate that did not run is not a ledger gate that found nothing, and this is
    # the file's own history: the check it wraps sat committed and uncalled for weeks
    # while the number it measures doubled.
    LEDGER_RC=$(sed -n 's/^LEDGER_EXIT=//p' "$LOG" | tail -n 1)
    [[ $LEDGER_RC =~ ^[0-9]+$ ]] \
        || die "--verdict-only: $LOG carries no \`LEDGER_EXIT=<n>\` line, the ledger gate was
       never measured, so there is no verdict to give over it."
    ROOT=$(stamp pwd);                 ROOT=${ROOT:-?}
    HEAD_SHA=$(stamp 'sigil HEAD');    HEAD_SHA=${HEAD_SHA:-?}
    local br; br=$(stamp 'sigil branch')
    BRANCH=${br% (*}; BRANCH=${BRANCH:-?}
    DIRTY=${br##*(}; DIRTY=${DIRTY%)}; [[ $br == *'('* ]] || DIRTY=?
    local ae; ae=$(stamp AEON_DIR)
    AEON=${ae% (step*}; AEON=${AEON:-?}
    AEON_HEAD=$(stamp 'aeon HEAD');    AEON_HEAD=${AEON_HEAD:-?}
    local ab; ab=$(stamp 'aeon branch')
    AEON_BRANCH=${ab% (*}; AEON_BRANCH=${AEON_BRANCH:-?}
    AEON_DIRTY=${ab##*(}; AEON_DIRTY=${AEON_DIRTY%)}; [[ $ab == *'('* ]] || AEON_DIRTY=?
    ROM_STATE=$(stamp 'aeon ROMs');    ROM_STATE=${ROM_STATE:-?}
    TARGET=$(stamp TARGET_DIR);        TARGET=${TARGET:-?}
    STARTED=$(stamp 'started (UTC)');  STARTED=${STARTED:-?}
    FINISHED=$(stamp 'finished (UTC)'); FINISHED=${FINISHED:-?}
    [[ $(stamp scoped) == YES* ]] && SCOPED=1
    # A --baseline on this command line wins; otherwise the one the run stated.
    if [[ -z $BASELINE ]]; then
        local sb; sb=$(stamp baseline)
        [[ $sb =~ ^[0-9]+$ ]] && BASELINE=$sb
    fi
    say "verdict-only: re-judging $LOG (tree $ROOT @ ${HEAD_SHA:0:8}), nothing runs"
}

# The run itself: preflight refusals, the stamp, the lint bar and the suite. One function
# so that --verdict-only can take the other branch of ONE dispatch below and reach the
# identical verdict code, rather than a copy of it. The body is the linear script it was
# and is left at its column so its history stays readable; every variable it sets is
# global, which is exactly what the verdict block reads.
run_landing() {
# ---------------------------------------------------------------------------------------
# (0) Where we are. Everything below is derived from this, never from the caller's cwd.
# ---------------------------------------------------------------------------------------
ROOT=$(git rev-parse --show-toplevel 2>/dev/null) \
    || die "not inside a git checkout, this must run from a sigil worktree"
ROOT=$(abspath "$ROOT")
[[ -f $ROOT/Cargo.toml ]] || die "$ROOT has no Cargo.toml, that is not the sigil workspace"

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
       Use a dedicated directory, the default \`$ROOT/.target-land\` is gitignored, or
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
    || die "cannot source $ROOT/scripts/lib/suite_paths.sh, the reference tree cannot be
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
[[ -f $AEON/build.sh ]] || die "AEON_DIR resolves to $AEON, which has no build.sh, that is
       not an aeon checkout. Pass --aeon <path to a built aeon checkout>."

# THE SECOND SIBLING, named by this script for the same reason the first one is. The M1.B
# listing gate compiles a micro-harness against the legacy Exodus port's `Symbols.cpp`, and
# that resolver refuses a tree nobody named exactly as the engine's does. Left unnamed, this
# run would stop inside the suite with the resolver's own message rather than here with a
# usable one, and before 2026-09-08 it did neither: the gate fell through to a fixed path
# and measured a peer's live checkout under a strict landing green.
#
# WHAT EXPORTING IT COSTS, stated because it is a real limit and not a detail. When the
# include answers at STEP 3, this script hands the child a derived tree under the name of an
# explicitly-chosen one, and from inside the suite the two are indistinguishable: the
# resolver sees a variable and cannot see who set it. That is the same trade the AEON_DIR
# line above already makes, and the same mitigation covers both, which is why the step is
# stamped into the log header rather than only the path. A reader asking WHICH VALUE WAS IN
# EFFECT gets it from the stamp; the suite cannot answer it on its own.
ORACLE_LEGACY=$(suite_resolve_checkout oracle-old ORACLE_DIR) \
    || die "the legacy oracle tree could not be resolved (see the refusal above).
       The M1.B listing gate compiles against its linux-port/gui/Symbols.cpp, so a landing
       run cannot measure that gate without one. Export ORACLE_DIR=<oracle-old checkout>."
ORACLE_LEGACY=$(abspath "$ORACLE_LEGACY")
if [[ -n ${ORACLE_DIR:-} ]]; then ORACLE_STEP="1: explicit ORACLE_DIR"
elif [[ -n ${EMPYREAN_SUITE_ROOT:-} ]]; then ORACLE_STEP="2: EMPYREAN_SUITE_ROOT/oracle-old"
else ORACLE_STEP="3: sibling of this checkout via git --git-common-dir"
fi

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

# (7) CLIPPY MUST EXIST BEFORE ANYTHING SPENDS TIME. `cargo clippy` on a toolchain
# without the component fails with a message about an unknown subcommand, and that exit
# code is indistinguishable in the verdict from a lint bar that ran and found errors.
# "The lint bar could not be measured" and "the lint bar is red" are different facts, so
# the unmeasurable one is refused here BY NAME rather than rendered as a lint count later.
CLIPPY_VERSION=$(cargo clippy --version 2>/dev/null) \
    || die "\`cargo clippy\` is not available on this toolchain, so the lint bar cannot be
       measured, and an unmeasurable bar is not a passing one. Install it with
       \`rustup component add clippy\` and re-run."
say "clippy available: $CLIPPY_VERSION"

# (8) THE LEDGER GATE MUST EXIST AND BE RUNNABLE BEFORE ANYTHING SPENDS TIME, for the
# reason clippy is checked above: "the gate could not be measured" and "the gate is red"
# are different facts, and only one of them is about the ledger. Refused here BY NAME
# rather than surfacing later as an exit code the verdict would have to guess at.
LEDGER_GATE=$ROOT/scripts/ledger_gate.py
[[ -f $LEDGER_GATE ]] \
    || die "the ledger gate is not at $LEDGER_GATE. It asserts what the owner's console can
       render out of docs/*.jsonl, and a landing that cannot run it is a landing with that
       question unanswered, not one with a clean answer."
PYTHON_VERSION=$(python3 --version 2>&1) \
    || die "\`python3\` is not on PATH, so the ledger gate cannot be measured, and an
       unmeasurable gate is not a passing one."
say "python3 available: $PYTHON_VERSION"

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
       file, the pre-flight build reported success without producing it."
[[ -x $SIGIL_EMIT_RESOLVED ]] \
    || die "SIGIL_EMIT is $SIGIL_EMIT_RESOLVED ($EMIT_ORIGIN), which is not an executable
       file, the pre-flight build reported success without producing it."

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
        say "WARNING, the reference tree $AEON is missing ${#MISSING_ROMS[@]} built ROM(s):
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

# THE LINT BAR'S OWN ARGUMENTS, and they do not follow --scoped. `--scoped` narrows which
# TESTS run; it says nothing about which code has to lint, and a scoped run that also
# narrowed the lint bar would be a second spelling of the omission this closes. The bar is
# the workspace, every target, warnings denied — the form README.md states.
CLIPPY_ARGS=(clippy --release --workspace --all-targets
             --manifest-path "$ROOT/Cargo.toml" -- -D warnings)

# THE LEDGER GATE'S ARGUMENTS, and they do not follow --scoped either, for a reason
# stronger than clippy's: this gate reads `docs/*.jsonl` in THIS checkout and consults no
# reference tree, so there is nothing about it a partial run could be partial about. NO
# `--pin` IS PASSED, deliberately -- the pin a landing enforces is the constant inside
# that script, where changing it is a diff someone reviews, and not a number a command
# line can move on the day it goes red.
LEDGER_ARGS=("$LEDGER_GATE" --repo "$ROOT" --docs "$ROOT/docs")

# ---------------------------------------------------------------------------------------
# THE TEST-TARGET CENSUS, and it is the only figure in this file that comes from OUTSIDE
# the log. The launch/report pairing in the verdict catches a binary that started and went
# quiet; it is SILENT about a target that stopped being built, because such a target
# neither launches nor reports and both halves of that pairing are read out of the log.
# `cargo metadata --no-deps` reads the manifests and builds nothing, so it says how many
# binaries a full run SHOULD launch without being able to be affected by what the run did.
#
# STAMPED INTO THE LOG rather than measured at verdict time, so `--verdict-only` judges the
# tree the run was made from and not whatever the manifests say today.
#
# A CENSUS THAT FAILED IS STAMPED AS SUCH, NEVER AS A NUMBER. It does not stop the run:
# the census is a cross-check on the other bars, and refusing to run the suite because a
# metadata call failed would trade a measurement for a missing one.
# ---------------------------------------------------------------------------------------
CENSUS_OUT=$(python3 "$ROOT/scripts/test_target_census.py" "$ROOT/Cargo.toml" 2>&1)
CENSUS_RC=$?
if (( CENSUS_RC == 0 )); then
    CENSUS_RUNNABLE=$(awk '$1=="runnable"{print $2}' <<< "$CENSUS_OUT")
    CENSUS_DOCTEST=$(awk '$1=="doctest"{print $2}' <<< "$CENSUS_OUT")
    CENSUS_EXCLUDED=$(awk '$1=="excluded-required-features"{print $2}' <<< "$CENSUS_OUT")
    CENSUS_LINE="$(( CENSUS_RUNNABLE + CENSUS_DOCTEST )) expected launches ($CENSUS_RUNNABLE runnable + $CENSUS_DOCTEST doctest, $CENSUS_EXCLUDED excluded for required-features; cargo metadata --no-deps)"
else
    CENSUS_LINE="COULD NOT MEASURE (scripts/test_target_census.py exited $CENSUS_RC: ${CENSUS_OUT//$'\n'/ })"
fi
say "test-target census: $CENSUS_LINE"

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
    echo "# ORACLE_DIR     $ORACLE_LEGACY (step $ORACLE_STEP)"
    echo "# TARGET_DIR     $TARGET"
    echo "# SIGIL_BUILD    $SIGIL_BUILD_RESOLVED ($BUILD_ORIGIN)"
    echo "# SIGIL_EMIT     $SIGIL_EMIT_RESOLVED ($EMIT_ORIGIN)"
    echo "# scoped         $( ((SCOPED)) && echo 'YES, this is a PARTIAL run, not a landing' || echo 'no (full workspace)')"
    echo "# baseline       ${BASELINE:-<none stated>}"
    echo "# test targets   $CENSUS_LINE"
    # The suite's own partial-run opt-in, stamped as CLEARED rather than left to be
    # assumed: the log has to be answerable about whether the reference-dependent rows
    # were measured, and "the variable was not set in my shell" is not something a
    # later reader of this file can check.
    echo "# allow-partial  removed from the child (was: ${SIGIL_ALLOW_PARTIAL:-<unset>})"
    echo "# clippy         $CLIPPY_VERSION"
    echo "# lint command   cargo ${CLIPPY_ARGS[*]}"
    echo "# python3        $PYTHON_VERSION"
    echo "# ledger command python3 ${LEDGER_ARGS[*]}"
    echo "# command        SIGIL_STRICT_GATE=1 env -u SIGIL_ALLOW_PARTIAL cargo ${CARGO_ARGS[*]}"
    echo
} > "$LOG" || die "cannot stamp the log $LOG"

say "log -> $LOG"
say "(tail it: tail -f $LOG)"

# ---------------------------------------------------------------------------------------
# (7) The lint bar, INSIDE this script's own command span. It runs FIRST because a lint
# error is a compile-time fact and a reader tailing the log should meet it in the first
# minute rather than the fifth — but a red bar does NOT stop the suite. The two are
# independent measurements and a landing wants both; short-circuiting here would hand back
# a verdict with the test half unmeasured, which is the shape (2) already refuses.
# ---------------------------------------------------------------------------------------
# ---------------------------------------------------------------------------------------
# (9) THE LEDGER GATE. Runs FIRST because it costs milliseconds and answers a question
# about this checkout alone, so a reader tailing the log meets it before the compile.
#
# THIS IS THE COLLECTING HALF ONLY. Nothing here aborts: `LEDGER_RC` is written to the
# log and execution continues into the lint bar and the suite, exactly as a red clippy
# does, because the two are independent measurements and a landing wants all of them. THE
# DECIDING HALF IS `LEDGER_RC != 0` IN THE `RESULT FAILED` CONDITION AT THE FOOT OF THIS
# FILE -- go and read it before concluding from this block that the gate is decorative.
# That misreading is the whole hazard of this shape and it is why the pointer is here.
# ---------------------------------------------------------------------------------------
echo "##### LEDGER SPAN, python3 ${LEDGER_ARGS[*]}" >> "$LOG"
say "ledger gate: python3 ${LEDGER_ARGS[*]}"
python3 "${LEDGER_ARGS[@]}" 2>&1 | tee -a "$LOG"
# PIPESTATUS[0], never `$?`, for the reason (6) gives: with `tee` in the pipeline `$?` is
# tee's status, and tee succeeds over a red gate.
LEDGER_RC=${PIPESTATUS[0]}
echo "LEDGER_EXIT=$LEDGER_RC" >> "$LOG"
echo "##### LEDGER SPAN ENDS" >> "$LOG"

echo "##### CLIPPY SPAN, cargo ${CLIPPY_ARGS[*]}" >> "$LOG"
say "lint bar: cargo ${CLIPPY_ARGS[*]}"
CARGO_TARGET_DIR="$TARGET" cargo "${CLIPPY_ARGS[@]}" 2>&1 | tee -a "$LOG"
# PIPESTATUS[0] for the same reason (6) gives: with `tee` in the pipeline `$?` is tee's.
CLIPPY_RC=${PIPESTATUS[0]}
echo "CLIPPY_EXIT=$CLIPPY_RC" >> "$LOG"
echo "##### CLIPPY SPAN ENDS" >> "$LOG"

# ---------------------------------------------------------------------------------------
# (3)+(6) The run. The strict flag is INSIDE the command span; the exit code is cargo's.
# ---------------------------------------------------------------------------------------
echo "##### TEST SPAN, cargo ${CARGO_ARGS[*]}" >> "$LOG"
SIGIL_STRICT_GATE=1 \
CARGO_TARGET_DIR="$TARGET" \
AEON_DIR="$AEON" \
ORACLE_DIR="$ORACLE_LEGACY" \
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
}

# THE ONE DISPATCH. Both branches leave the same variables set and fall through to the
# same verdict code; there is no second verdict.
if [[ -n $VERDICT_ONLY ]]; then
    load_verdict_inputs "$VERDICT_ONLY"
else
    run_landing
fi

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
LEDGER_SILENT=0
# Every lint site, named. `error: could not compile …` is clippy's TALLY line, not a
# finding, so counting bare `^error:` reports one more site than exists, and a verdict
# that cannot be checked against the log by hand is a verdict a reader has to trust.
# Parsed here, beside the other log readers, because it reads the log and nothing else:
# the verdict-only path has no clippy process to ask.
mapfile -t CLIPPY_SITES < <(awk '
    /^##### CLIPPY SPAN ENDS/ { inspan = 0 }
    inspan && /^error: / && !/^error: could not compile/ { msg = substr($0, 8); next }
    inspan && /^ *--> / && msg != "" {
        loc = $2
        print loc "  " msg
        msg = ""
    }
    /^##### CLIPPY SPAN,/ { inspan = 1 }' "$LOG")
# The ledger gate's own report, lifted out of ITS span so the number it measured is
# visible in the verdict a merge reads rather than only in the log body. Scoped to the
# span for the same reason the skip counter is: `LEDGER:` is a literal a lint or a test
# could quote, and a matcher that read one as a measurement would be reporting a figure
# nothing produced. The `LEDGER: ` prefix is stripped; the gate owns the wording.
mapfile -t LEDGER_LINES < <(awk '
    /^##### LEDGER SPAN ENDS/ { inspan = 0 }
    inspan && /^LEDGER:/ { print substr($0, 9) }
    /^##### LEDGER SPAN,/ { inspan = 1 }' "$LOG")

# BOTH spellings. The landing bar greps `skip:`, and 27 sites say `skipping` instead —
# invisible to that grep while reporting green. A matcher inheriting the same blind spot
# would under-count while still looking like a witness.
# Counted ONLY inside the test span. This is the one matcher in the file loose enough to
# read the lint bar's output as suite output: clippy quotes source lines verbatim, so a
# lint fired on any code containing `skipping` would be counted here as a gate that
# measured nothing. The two spans now share a log, so the parser has to say which half it
# is reading. The three matchers below need no such scoping — `test result:`,
# `^test … FAILED`, and the --expect-test probe are all anchored on shapes cargo-test
# emits and clippy does not — and that is a fact about those patterns, so it is written
# here rather than left to be re-derived.
SKIPS=$(awk '
    /^##### TEST SPAN,/ { inspan = 1; next }
    inspan && /skip:|skipping/ { n++ }
    END { print n+0 }' "$LOG")

# ---------------------------------------------------------------------------------------
# (7b) WHICH BINARIES LAUNCHED, AND WHICH OF THEM REPORTED.
#
# `SUITES` above counts `test result:` lines and NOTHING SAYS WHAT THAT COUNT SHOULD BE.
# A test binary that dies mid-run contributes no `test result:` line at all, so its tests
# leave the totals silently: `suites` and `passed` both come back smaller and every line
# in the verdict still reads like a complete run. Measured on this repo, from a real green
# log with one binary's report removed: 449 suites became 448, 5077 passed became 5075,
# and the verdict printed `RESULT GREEN` with `5063 baseline + 12 new = 5075 observed`.
#
# THE BASELINE DOES NOT COVER THIS. `--baseline` is the count expected on a green run and
# the reconciliation only fails on a SHORTFALL against it, so it is a lower bound with
# slack: every test added since the caller last updated the number is slack a vanished
# test can hide inside. Measured across the seventeen landing logs in this repo the slack
# ran from 0 to 21 tests, and stood at 14 on the day this was written.
#
# THE POPULATION COMES FROM A DIFFERENT PRODUCER THAN THE REPORT, which is the whole point
# and the reason this is not a set checked against itself. `Running <target>` and
# `Doc-tests <crate>` are written by CARGO, the parent, before each child starts;
# `test result:` is written by the CHILD when it finishes. A child that dies cannot
# retract a line its parent already flushed, so the launch record survives exactly the
# failure this is looking for. Measured on all seventeen logs: launched == reported in
# every one, including the red one, so this is an equality and not a ratchet.
#
# The pairing, rather than two counts subtracted, so the verdict can NAME the binary that
# went quiet: a count tells an operator a target died and leaves them to find which.
LAUNCHED=$(awk '
    /^##### TEST SPAN,/ { inspan = 1; next }
    inspan && (/^ *Running / || /^ *Doc-tests /) { n++ }
    END { print n+0 }' "$LOG")
mapfile -t SILENT_BINARIES < <(awk '
    /^##### TEST SPAN,/ { inspan = 1; next }
    !inspan { next }
    /^ *Running / || /^ *Doc-tests / {
        if (pending != "") print pending
        sub(/^ +/, ""); pending = $0
        next
    }
    /^test result:/ { pending = "" }
    END { if (pending != "") print pending }' "$LOG")
# The third state, and it is LOUD rather than 0. A log whose test span reports suites but
# records no launches is a log this check cannot measure, and rendering an unmeasurable
# population as a satisfied one is the defect this whole block exists to close.
COMPLETENESS_UNMEASURED=0
(( SUITES > 0 && LAUNCHED == 0 )) && COMPLETENESS_UNMEASURED=1

# THE CENSUS, read back out of the stamp. This is the half the pairing above cannot do: a
# target that stopped being built neither launches nor reports, so it is absent from both
# of the log's own populations and only a figure derived from the manifests can miss it.
# Three states, and the third is why this is parsed rather than defaulted:
#   a number  -- the run stamped a census, compare it to what launched
#   COULD...  -- the run tried and the census failed, which is not a satisfied cross-check
#   empty     -- the log predates this stamp entirely, so there is nothing to compare
CENSUS_STAMP=$(stamp 'test targets')
CENSUS_EXPECTED=${CENSUS_STAMP%% *}
CENSUS_STATE=absent
[[ -n $CENSUS_STAMP ]] && CENSUS_STATE=unmeasured
[[ $CENSUS_EXPECTED =~ ^[0-9]+$ ]] && CENSUS_STATE=measured
CENSUS_SHORT=0
if [[ $CENSUS_STATE == measured ]] && (( ! SCOPED )) && (( LAUNCHED != CENSUS_EXPECTED )); then
    CENSUS_SHORT=1
fi
CENSUS_UNMEASURED=0
[[ $CENSUS_STATE == unmeasured ]] && CENSUS_UNMEASURED=1

# Every failing name, sorted and deduped. All of them.
mapfile -t FAILING < <(grep -E '^test .* \.\.\. FAILED$' "$LOG" \
    | sed -E 's/^test (.*) \.\.\. FAILED$/\1/' | sort -u)

echo
echo "=============================== LANDING RUN VERDICT ==============================="
echo "  log             $LOG"
echo "  tree            $ROOT @ ${HEAD_SHA:0:8} ($BRANCH, $DIRTY)"
echo "  reference       $AEON @ ${AEON_HEAD:0:8} ($AEON_BRANCH, $AEON_DIRTY), $ROM_STATE"
echo "  target dir      $TARGET"
echo "  started/ended   $STARTED -> $FINISHED (UTC)"
echo "  CARGO_EXIT      $CARGO_RC"
echo "  CLIPPY_EXIT     $CLIPPY_RC   ($( (( CLIPPY_RC == 0 )) && echo 'lint bar clean' || echo "LINT BAR RED, ${#CLIPPY_SITES[@]} site(s)" ))"
case $LEDGER_RC in
    0) echo "  LEDGER_EXIT     $LEDGER_RC   (ledger gate clean)" ;;
    2) echo "  LEDGER_EXIT     $LEDGER_RC   (LEDGER GATE UNMEASURABLE, which is not a clean ledger)" ;;
    *) echo "  LEDGER_EXIT     $LEDGER_RC   (LEDGER GATE RED)" ;;
esac
(( SCOPED )) && echo "  ** SCOPED RUN, PARTIAL. This is not a landing verdict. **"

# ALWAYS PRINTED, GREEN OR RED. The renderability figure is a TREND and a verdict that
# showed it only when it broke would hide the two things worth watching: a count that
# crept and a pin that could tighten. It is four lines when everything holds.
if (( ${#LEDGER_LINES[@]} )); then
    echo
    echo "  LEDGER GATE (docs/*.jsonl), all of it:"
    for l in "${LEDGER_LINES[@]}"; do echo "    $l"; done
else
    # A `LEDGER_EXIT=` line with no report above it. THIS FAILS THE RUN rather than
    # warning beside it: an emptiness is not a finding without an instrument that could
    # have returned non-empty, and a gate that exits 0 having printed nothing is
    # indistinguishable from one that measured nothing. The real gate cannot reach this
    # -- it reports before it can exit 0 -- so the only way here is a gate that broke or
    # a log that lost its span, and neither is a landing.
    LEDGER_SILENT=1
    echo
    echo "  LEDGER GATE PRODUCED NO REPORT LINES, only an exit code $LEDGER_RC. Read the"
    echo "  LEDGER SPAN in the log. A silent gate is not a clean one, and this run is not"
    echo "  green on the strength of an exit code with no measurement behind it."
fi

# Before the unmeasurable branches below, because a run whose tests could not be measured
# still measured the lint bar, and dropping that finding on the way out would make the
# operator run the whole thing again to learn something already known.
if (( ${#CLIPPY_SITES[@]} )); then
    echo
    echo "  CLIPPY LINT ERRORS (${#CLIPPY_SITES[@]}), all of them:"
    for s in "${CLIPPY_SITES[@]}"; do echo "    $s"; done
    echo "  THIS LIST IS A LOWER BOUND, NOT THE POPULATION. Under \`-D warnings\` the first"
    echo "  crate to fail aborts and cargo stops scheduling the rest, so every unchecked"
    echo "  target contributes nothing here. Measured on this repo: 10 sites, then 35 more,"
    echo "  then 71 more, as each round of fixes let the next target compile. To size the"
    echo "  work, re-run the same command WITHOUT \`-- -D warnings\`, nothing aborts and"
    echo "  everything is checked, and count from that."
    echo "  Silence the SPECIFIC item with a comment saying why, or change the code. A"
    echo "  workspace-wide or crate-wide allow turns a correct lint off everywhere to"
    echo "  settle one site, and is not a fix."
elif (( CLIPPY_RC != 0 )); then
    echo
    echo "  CLIPPY EXITED $CLIPPY_RC WITH NO PARSEABLE LINT SITE. The bar is red for a"
    echo "  reason this verdict could not name, read the CLIPPY SPAN in the log. Do not"
    echo "  read an unnamed red as a lint that can be waived."
fi

# UNMEASURABLE IS NOT GREEN. Both branches below are runs whose result cannot be
# classified, and neither may be rendered as a count.
if (( SUITES == 0 )); then
    echo "  RESULT          COULD NOT RUN, no \`test result:\` line in the log."
    echo "                  Nothing about this run can be measured. cargo exited $CARGO_RC."
    echo "==================================================================================="
    exit 2
fi
if (( PASSED == 0 )); then
    echo "  RESULT          COULD NOT RUN, $SUITES suite(s) reported, 0 tests passed."
    echo "                  That is a run that could not be measured, not a green one."
    echo "==================================================================================="
    exit 2
fi

echo "  suites          $SUITES"
echo "  passed          $PASSED"
echo "  failed          $FAILED"
echo "  ignored         $IGNORED"
if (( SKIPS > 0 )); then
    echo "  skip lines      $SKIPS   <-- FAILS THIS RUN. SIGIL_STRICT_GATE=1 should make these"
    echo "                          impossible. Each is a gate that measured nothing while"
    echo "                          reporting green, so the passes above are not a landing."
    echo "                          Grep the log for 'skip:' and 'skipping'."
else
    echo "  skip lines      0"
fi

# ALWAYS PRINTED, GREEN OR RED, for the reason the ledger report is: a completeness figure
# shown only when it breaks leaves a reader unable to tell a run that checked from a run
# that did not.
if (( COMPLETENESS_UNMEASURED )); then
    echo "  binaries        COULD NOT MEASURE. $SUITES suite(s) reported and the test span"
    echo "                  records no \`Running\`/\`Doc-tests\` launch line at all, so nothing"
    echo "                  here knows how many binaries SHOULD have reported. This is not a"
    echo "                  satisfied completeness check, it is an absent one."
elif (( ${#SILENT_BINARIES[@]} )); then
    echo "  binaries        $LAUNCHED launched, $SUITES reported   <-- FAILS THIS RUN."
    echo
    echo "  BINARIES THAT LAUNCHED AND NEVER REPORTED (${#SILENT_BINARIES[@]}), all of them:"
    for b in "${SILENT_BINARIES[@]}"; do echo "    $b"; done
    echo "  Each of these started and produced no \`test result:\` line, so however many tests"
    echo "  it holds are absent from the counts above and the totals still read complete. A"
    echo "  binary killed by the OOM killer leaves exactly this trace; so does one that"
    echo "  aborted, hung until something killed it, or died in a static initialiser. Read"
    echo "  the log AT THE NAMED TARGET, not at the tail."
else
    echo "  binaries        $LAUNCHED launched, $SUITES reported"
fi

# THE ONE FIGURE FROM OUTSIDE THE LOG. Always printed, for the reason the ledger report is.
case $CENSUS_STATE in
    measured)
        if (( SCOPED )); then
            echo "  test targets    $CENSUS_STAMP"
            echo "                  REPORTED, NOT CHECKED. A --scoped run launches a subset by"
            echo "                  design, so a shortfall against the census is the point of the"
            echo "                  flag rather than a finding."
        elif (( CENSUS_SHORT )); then
            echo "  test targets    $CENSUS_STAMP"
            if (( LAUNCHED < CENSUS_EXPECTED )); then
                echo "                  <-- FAILS THIS RUN. $(( CENSUS_EXPECTED - LAUNCHED )) target(s) the manifests"
                echo "                  describe never launched at all. The pairing above cannot see this:"
                echo "                  a target that stopped being built neither launches nor reports, so"
                echo "                  it is absent from both populations the log carries. Something was"
                echo "                  removed from a Cargo.toml, gained a \`test = false\`, or stopped"
                echo "                  compiling into a target cargo would run."
            else
                echo "                  <-- FAILS THIS RUN. $(( LAUNCHED - CENSUS_EXPECTED )) MORE launch(es) than the"
                echo "                  manifests describe. The census derivation and cargo disagree, and"
                echo "                  until they are reconciled neither number can be trusted as the"
                echo "                  population. Read scripts/test_target_census.py against the"
                echo "                  manifests; a target kind it does not know about is the likely gap."
            fi
        else
            echo "  test targets    $CENSUS_STAMP, all launched"
        fi
        ;;
    unmeasured)
        echo "  test targets    $CENSUS_STAMP"
        echo "                  <-- FAILS THIS RUN. The census is the only population in this"
        echo "                  verdict that does not come out of the log, and a run that could"
        echo "                  not take it has no outside check on what was built at all. This"
        echo "                  is an absent cross-check, not a satisfied one."
        ;;
    absent)
        echo "  test targets    NOT STATED, this log predates the census stamp. Nothing outside"
        echo "                  the log says how many binaries should have launched, so the"
        echo "                  launch/report pairing above is the only completeness evidence"
        echo "                  here. A run made by this script today always stamps one."
        ;;
esac

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
        echo "  baseline        INVALID (\`$BASELINE\` is not a number), nothing reconciled."
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
            echo "                  them, these did not run at all. Something stopped being"
            echo "                  built, was filtered out, or was marked #[ignore]."
            RECONCILED=0
        fi
    fi
else
    echo "  reconciles      NOT CHECKED, no --baseline stated. A bare pass count is not a"
    echo "                  result; state the baseline you expect."
fi

# ---------------------------------------------------------------------------------------
# The verdict. Derived from cargo's own status and the measurement — never from whatever
# command happened to run last.
# ---------------------------------------------------------------------------------------
# A RED LINT BAR IS A RED RUN. It sits in the same condition as a red test rather than in
# a warning line above it, because a bar reported beside a `RESULT GREEN` is a bar that
# gets landed over — which is how ten lint errors reached master under a wrapper that
# printed GREEN.
# A SKIP LINE IS A RED RUN, for the same reason and in the same condition. The count was
# printed as a WARNING beside `RESULT GREEN` and exit 0, which is the identical shape:
# four documents said the landing bar fails on a skip line, and the wrapper did not.
#
# A RED LEDGER GATE IS A RED RUN, in this same condition and for the third time the same
# reason. THIS IS THE DECIDING HALF OF (8): the measuring half is the `(9) THE LEDGER
# GATE` block above, which deliberately does not abort, and if you arrived here from
# there this line is what makes that block load-bearing. `LEDGER_RC` covers red (1) and
# unmeasurable (2) alike -- an unmeasurable gate is not a passing one -- and
# `LEDGER_SILENT` covers the third state, a gate that returned an exit code with no
# measurement behind it.
#
# A BINARY THAT LAUNCHED AND NEVER REPORTED IS A RED RUN, in this same condition and for
# the fourth time the same reason. Its tests are missing from every total above while
# every total still reads complete, and cargo's exit code covers only the half of that
# where cargo noticed: a binary killed by a signal makes cargo exit 101 (measured), but a
# target that stopped being built, was filtered out, or was marked `#[ignore]` takes tests
# out of the count with cargo exiting 0. The condition below is what makes (7b) load
# bearing rather than a line in a verdict block. `COMPLETENESS_UNMEASURED` covers the
# third state, a run whose population this check could not establish at all.
#
# A CENSUS SHORTFALL IS A RED RUN, and a census that could not be taken is one too. The
# `absent` state is NOT in this condition, deliberately: it is a log written before the
# stamp existed, and a rule that failed every historical log would be a rule people learn
# to route around rather than a bar.
if (( CARGO_RC != 0 || FAILED > 0 || CLIPPY_RC != 0 || SKIPS > 0 || LEDGER_RC != 0 || LEDGER_SILENT \
      || ${#SILENT_BINARIES[@]} > 0 || COMPLETENESS_UNMEASURED || CENSUS_SHORT || CENSUS_UNMEASURED )); then
    echo
    if (( LEDGER_RC != 0 && CARGO_RC == 0 && FAILED == 0 && SKIPS == 0 && CLIPPY_RC == 0 )); then
        # The informative case again: nothing about the code is red, and the run still is
        # not a landing because the owner's decision ledger moved somewhere he cannot read.
        if (( LEDGER_RC == 2 )); then
            echo "  RESULT          FAILED, the LEDGER GATE could not measure (exit 2). Every test"
            echo "                  that ran passed and the lint bar is clean; the ledger question is"
            echo "                  UNANSWERED, which is not the same as answered clean. Do not land"
            echo "                  on this."
        else
            echo "  RESULT          FAILED, the LEDGER GATE is red (exit $LEDGER_RC). Every test that ran"
            echo "                  passed and the lint bar is clean; the suite is not the reason this"
            echo "                  is not green. The report above names the lines. Do not land on this."
        fi
    elif (( LEDGER_SILENT && CARGO_RC == 0 && FAILED == 0 && SKIPS == 0 && CLIPPY_RC == 0 )); then
        echo "  RESULT          FAILED, the LEDGER GATE reported nothing. Every other bar is clean;"
        echo "                  a gate with no measurement behind its exit code is why this is not"
        echo "                  green. Do not land on this."
    elif (( CLIPPY_RC != 0 && CARGO_RC == 0 && FAILED == 0 && SKIPS == 0 && LEDGER_RC == 0 && ! LEDGER_SILENT )); then
        # Named separately because the two halves disagreeing is the informative case, and
        # "$FAILED test(s) red" printed as 0 over a red run reads as a script mistake.
        echo "  RESULT          FAILED, the LINT BAR is red (clippy exit $CLIPPY_RC,"
        echo "                  ${#CLIPPY_SITES[@]} site(s)). Every test that ran passed; the suite is not"
        echo "                  the reason this is not green. Do not land on this."
    elif (( SKIPS > 0 && CARGO_RC == 0 && FAILED == 0 && CLIPPY_RC == 0 && LEDGER_RC == 0 && ! LEDGER_SILENT )); then
        # The same informative case for the third bar: nothing was red, and the run is
        # still not a landing because $SKIPS gate(s) never measured their subject.
        echo "  RESULT          FAILED, $SKIPS skip line(s) survived SIGIL_STRICT_GATE=1. Every test"
        echo "                  that ran passed and the lint bar is clean; a gate that measured"
        echo "                  nothing is why this is not green. Do not land on this."
    elif (( ${#SILENT_BINARIES[@]} > 0 && CARGO_RC == 0 && FAILED == 0 && SKIPS == 0 && CLIPPY_RC == 0 && LEDGER_RC == 0 && ! LEDGER_SILENT )); then
        # The informative case for the fourth bar, and the one worth naming loudest: every
        # bar the run measures is clean and the run is still not a landing, because the
        # POPULATION it measured them over is short. Nothing else in this verdict would
        # have said so -- the totals shrink silently and the baseline has slack.
        echo "  RESULT          FAILED, ${#SILENT_BINARIES[@]} test binary/binaries launched and never"
        echo "                  reported. Every test that ran passed, the lint bar is clean and cargo"
        echo "                  exited 0; the tests inside those binaries are simply not in the counts"
        echo "                  above. This is not a green run with a smaller number, it is a run whose"
        echo "                  population is unknown. Do not land on this."
    elif (( COMPLETENESS_UNMEASURED && CARGO_RC == 0 && FAILED == 0 && SKIPS == 0 && CLIPPY_RC == 0 && LEDGER_RC == 0 && ! LEDGER_SILENT )); then
        echo "  RESULT          FAILED, the completeness check could not measure. Every other bar is"
        echo "                  clean; this log records no binary launches, so how many binaries should"
        echo "                  have reported is unknown, and an unknown population is not a green one."
    elif (( CENSUS_SHORT && CARGO_RC == 0 && FAILED == 0 && SKIPS == 0 && CLIPPY_RC == 0 && LEDGER_RC == 0 && ! LEDGER_SILENT && ${#SILENT_BINARIES[@]} == 0 )); then
        # The informative case for the census: every binary that launched also reported,
        # so the log is internally consistent and still describes fewer targets than the
        # manifests do. Only a figure from outside the log can say that.
        echo "  RESULT          FAILED, the TEST-TARGET CENSUS does not match. $LAUNCHED binaries"
        echo "                  launched, the workspace manifests describe $CENSUS_EXPECTED. Every one that"
        echo "                  launched reported and every test in them passed, so the log agrees"
        echo "                  with itself; it is the workspace it does not agree with. Do not land"
        echo "                  on this."
    else
        echo "  RESULT          FAILED, $FAILED test(s) red, ${#SILENT_BINARIES[@]} silent binary/binaries, $SKIPS skip line(s), cargo exit $CARGO_RC, clippy exit $CLIPPY_RC, ledger exit $LEDGER_RC, census $CENSUS_STATE."
    fi
    echo "==================================================================================="
    exit 1
fi
if (( ! RECONCILED )); then
    echo
    echo "  RESULT          GREEN BUT UNRECONCILED, every test that ran passed, and the"
    echo "                  population is not the one you stated. Do not land on this."
    echo "==================================================================================="
    exit 3
fi
echo
echo "  RESULT          GREEN"
echo "==================================================================================="
exit 0
