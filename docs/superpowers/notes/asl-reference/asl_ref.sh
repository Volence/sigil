#!/usr/bin/env bash
# asl_ref.sh — select the reference assembler, and REFUSE any other build.
#
# Source it, do not run it, and PROPAGATE ITS STATUS — these runners do not set
# `-e`, so a sourced guard that merely returns non-zero stops nothing:
#
#     . "$(dirname "$0")/../asl-reference/asl_ref.sh" || exit $?
#     # $ASLDIR is now a directory whose `asl` is the reference build,
#     # $ASL is that binary, and AS_MSGPATH is exported to match.
#     asl_run -xx -n -q -A -L -U -i . probe.asm     # THE BLESSED INVOCATION
#
# Without the `|| exit $?` the runner assembles with a refused binary and the
# guard is decoration. `selfcheck.sh` case 4 is the one that catches dropping it.
#
# `asl_run` is defined at the bottom of this file and is the invocation to use.
# Calling `"$ASL"` directly still works and is UNBLESSED: it identifies the
# program and asks nothing about whether the program answered. See `asl_run`'s
# own header for what that costs, and `selfcheck.sh` cases 6 to 9 for the proof
# that the check fires.
#
# ── WHY A DIGEST AND NOT A VERSION STRING ────────────────────────────────────
# THE REFERENCE ASSEMBLER IS A BUILD, NOT A VERSION. Four `asl` binaries in this
# workspace print
#
#     Macro Assembler 1.42 Beta [Bld 212]
#
# verbatim, and they are not the same program. `s2disasm`'s Linux-x86_64 build
# (md5 0dee1f98e6480a4783d27ffd8b90896f, the flamewing fork) substitutes an
# UNINITIALIZED MEMORY WORD for any operand it declined to give a value, so it
# answers differently on every run — measured on `2026-09-04-as-end-probes`'
# own `wrange.asm`, where the four range-refused immediates come back
#
#     303C 5602 / 303C 55B1 / 303C 5655 / 303C 557F     (four consecutive runs)
#
# while `s1disasm`'s upstream build (md5 61e672562465725a8c102288a7da9098)
# returns `303C 8000` every time. A runner that checked the banner would have
# accepted either and verified nothing; that is the whole reason this file
# exists rather than a `--version` grep.
#
# ── AND THE REFERENCE BUILD IS NOT ANSWERING EITHER ──────────────────────────
# That `303C 8000` is NOT this build's answer for those lines, and this comment
# used to say "answers", which was wrong. It is line 5 of `wrange.asm` leaking
# downward: line 5 is `move.w #-32768,d0`, which is in range, is ACCEPTED, and
# legitimately computes $8000 — and the four refused lines below it echo THE
# LAST VALUE ASL COMPUTED. Change line 5's accepted value and all four move with
# it: that is `wcarry.asm`, beside `wrange.asm`, where they read `303C 1234`.
# With no accepted immediate above them at all (`wcarry0.asm`) they read `0000`,
# the slot's initial state.
#
# So the two builds do not differ in WHETHER they substitute, only in WHAT: this
# one substitutes a stale value, the other an uninitialized one. The stale one is
# the more dangerous to inherit, because re-running it agrees with itself and so
# reads like a measurement. THIS GUARD PINS WHICH BUILD ANSWERED; it cannot tell
# you the build answered at all. For a shape asl declines, the byte column is an
# artifact on either build — do not pin it.
#
# ── THE DIGEST IS A LITERAL, ON PURPOSE ──────────────────────────────────────
# `ASL_REF_MD5` below is written out. It is NOT computed from the binary the
# guard is about to check, because a guard whose expectation is derived from its
# subject moves with the subject and can never come out red — the same defect
# shape as a fixture built from the constant under test.
#
# ── HOW TO SEE IT FIRE ───────────────────────────────────────────────────────
# `selfcheck.sh` beside this file points the guard at the varying build and
# requires a refusal. A guard never seen to refuse is exactly the artifact this
# is here to remove, so run that, do not take this comment's word for it.
#
# `ASLDIR` may be overridden — pointing it at any build but the reference one is
# how the refusal is demonstrated, and the guard refuses it just the same. There
# is deliberately no escape hatch: a runner that can be talked into the varying
# build has the defect back.

ASL_REF_MD5=61e672562465725a8c102288a7da9098
ASL_REF_DIR=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64

ASLDIR="${ASLDIR:-$ASL_REF_DIR}"
ASL="$ASLDIR/asl"

if [ ! -x "$ASL" ]; then
    echo "FATAL: no executable asl at $ASL" >&2
    return 2 2>/dev/null || exit 2
fi

ASL_REF_GOT="$(md5sum "$ASL" | cut -d' ' -f1)"
if [ "$ASL_REF_GOT" != "$ASL_REF_MD5" ]; then
    echo "FATAL: $ASL is not the reference assembler." >&2
    echo "  want md5 $ASL_REF_MD5  ($ASL_REF_DIR/asl)" >&2
    echo "  got  md5 $ASL_REF_GOT" >&2
    echo "  The banner cannot tell these builds apart — all of them print" >&2
    echo "  'Macro Assembler 1.42 Beta [Bld 212]' — so the digest is the check." >&2
    return 3 2>/dev/null || exit 3
fi

AS_MSGPATH="$ASLDIR"
export AS_MSGPATH

# ── AND WHETHER IT ANSWERED: `asl_run` ───────────────────────────────────────
# THE MD5 SAYS WHICH PROGRAM RAN. THE EXIT STATUS SAYS WHETHER ITS ANSWERS MEAN
# ANYTHING. Everything above this line answers only the first question, and the
# header above says so in its own words: this guard pins which build answered,
# it cannot tell you the build answered at all.
#
# WHY THAT GAP IS NOT ACADEMIC, measured on `partial_failure.asm` beside this
# file. That probe has ONE invalid line, `bra.s /`, where `/` is a nameless
# label DEFINITION in AS and not a reference. Everything else in it is valid.
# asl reports the one error, exits 2, and prints a FULL BYTE COLUMN for every
# other line. One of those other lines is wrong because of it: the macro's
# `beq.s +` comes back
#
#     67FE      with the bad line present   (a branch to ITSELF)
#     6702      with the bad line deleted   (the correct forward branch)
#
# The listing looks complete. The corrupted value is plausible, in range, and
# the right shape, and nothing in the listing announces it. A reader who had
# pinned the digest perfectly and quoted 67FE would have carried a fabricated
# number while obeying every rule written above.
#
# So a run carrying ANY error is not a source of values for the lines that DID
# assemble. `asl_run` runs the pinned binary, reports the exit status in the
# transcript whether or not the caller thought to look, refuses out loud on a
# non-zero status, and RETURNS that status so a caller's `|| exit $?` works the
# same way it does for the digest check.
#
# WHAT `asl_run` STILL CANNOT TELL YOU, and this is the honest limit rather than
# a caveat. A ZERO EXIT IS NOT SUFFICIENT EITHER. For an operand this build
# DECLINES to value, it substitutes the last value it computed, exits 0, and
# prints no diagnostic at all: that is the `303C 8000` finding in the header
# above, where four range-refused immediates echo an accepted line five rows up.
# Digest plus exit status together answer "which program ran" and "did the run
# as a whole fail". THEY DO NOT ANSWER "did the build answer THIS line", and no
# check in this file does. For a shape asl declines, the byte column is an
# artifact on a clean exit too. Do not pin it.
#
# TWO MORE THINGS IT DOES NOT DO. It cannot make itself get used: a script that
# sources this guard and then calls `"$ASL"` directly gets none of it, and
# nothing here reddens when that happens. And a caller who redirects `asl_run`'s
# stderr to a log or to /dev/null loses the banner; the RETURN STATUS survives
# that, which is why the status and not the banner is the load-bearing half.
asl_run() {
    if [ -z "${ASL:-}" ]; then
        echo "FATAL: asl_run called with no \$ASL; source asl_ref.sh first." >&2
        return 2
    fi
    # Asked as a condition, not as a bare command, so that a caller running
    # under `set -e` reaches the report instead of dying at the assembler.
    local asl_rc=0
    if "$ASL" "$@"; then
        asl_rc=0
    else
        asl_rc=$?
    fi
    echo "ASL_EXIT=$asl_rc" >&2
    if [ "$asl_rc" -ne 0 ]; then
        echo "REFUSED: asl exited $asl_rc, so this run FAILED." >&2
        echo "  NO BYTE COLUMN FROM THIS RUN IS A SOURCE OF VALUES, including the" >&2
        echo "  lines that assembled: one error changes what OTHER lines encode to," >&2
        echo "  and the listing prints them looking complete. See partial_failure.asm" >&2
        echo "  beside asl_ref.sh, where a single bad line turns 6702 into 67FE." >&2
        echo "  Fix the source, do not quote the listing." >&2
    fi
    return "$asl_rc"
}
