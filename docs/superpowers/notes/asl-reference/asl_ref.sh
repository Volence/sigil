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
# other line. One of those other lines is wrong: the macro's `beq.s +` comes back
#
#     67FE      with the bad line present   (a branch to ITSELF)
#     6702      with the bad line deleted   (the correct forward branch)
#
# AND THE REASON IS NOT "THE ERROR CHANGED THAT LINE", which is what this comment
# used to say. `67FE` is the PASS-1 PLACEHOLDER for a forward reference asl had
# not resolved yet, and it survives because the error stopped the PASS LOOP
# before pass 2 could resolve it. Distinguishable, and distinguished: put an
# unknown-instruction error on line 3 of a file containing only that macro, with
# no `bra.s /` anywhere and nothing near the branch, and `beq.s +` still reads
# `67FE`. Any error, at any position, related or not. The rule below does not
# move; only its reason does.
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
#
# ── AND WHETHER IT LOOKED AT ALL: `asl_diag_state` ───────────────────────────
# A THIRD QUESTION, AND THE EXIT STATUS CANNOT ANSWER IT EITHER. asl assembles
# in a PASS LOOP. A forward reference is legal, so an undefined symbol is a
# provisional value on pass 1 and becomes `error #1010: symbol undefined` only
# when a LATER pass finds it still undefined. If ANY error occurs, asl refuses
# to start the next needed pass, and every diagnostic that pass would have
# raised is simply never looked for.
#
# So a run can fail for reason X and report NOTHING WHATEVER about reason Y,
# with a plausible failure and a plausible count. Measured beside this file on
# `swallowed_undef.asm` and `swallowed_undef_control.asm`, which differ in
# exactly one line: the subject carries `zzbogus d0,d1` above three undefined
# symbols, the control does not, and every other line is identical:
#
#     control  (3 undefined, nothing else wrong)   3 x #1010    exit 2, 2 passes
#     subject  (1 unrelated error + the same 3)    0 x #1010    exit 2, 1 pass
#
# BOTH ARMS FAIL, and the footer of the second reads `1 error`. A caller keyed
# to "did it fail" sees two failures and cannot tell them apart. POSITION IS
# IRRELEVANT: the same three vanish whether the unrelated error is above them or
# below them, because what stops is the pass LOOP, not the reading of the file.
#
# asl does say so, at the very bottom of the listing, in prose no runner parses:
#
#     1 pass
#       Additional necessary passes not started due to
#       errors, listing possibly incorrect.
#
# THE HONEST LIMIT, and it is the whole shape of this check: finding that line
# makes the incompleteness VISIBLE. IT DOES NOT MAKE THE MISSING DIAGNOSTICS
# APPEAR. A listing carrying it is not a smaller diagnostic set to be topped up;
# it is a set of unknown size. The only way to learn what was suppressed is to
# fix the reported error and assemble again.
#
# THREE STATES, NOT TWO, and the third is why this is a classifier and not a
# predicate. `complete` needs the footer to be PRESENT and to lack the line:
# absence of the line is not evidence of completeness, because a fatal error
# (exit 3) or a crash writes a listing with NO FOOTER AT ALL, and greping such a
# listing for the line finds nothing and would read as clean. 18 committed
# probes in this tree are that shape.
#
#   INCOMPLETE  the line is there: later-pass diagnostics were never looked for
#   complete    footer present, line absent: asl ran every pass it wanted
#   nofooter    a listing with no footer: the run died, completeness unknowable
#   missing     no listing to read at all (no `-L`, or a path this cannot derive)
#
# It reports and does not gate. It NEVER changes `asl_run`'s return status, and
# that is deliberate: many probes in this tree are supposed to fail, and several
# read a non-zero exit as the answer. A check that reddened those would be the
# always-red shape this repo rejects. `INCOMPLETE` also cannot occur on a clean
# run, because the line says "due to errors" and an error is exit 2, so nothing here
# fires on correct input.
asl_diag_state() {
    local lst="$1"
    if [ -z "$lst" ] || [ ! -f "$lst" ]; then
        echo missing
        return 0
    fi
    # ANCHORED, and it has to be. A listing ECHOES THE SOURCE, so a file whose
    # comments discuss this footer line contains the phrase in its own listing
    # and an unanchored `grep -q` calls it INCOMPLETE. That is not theoretical:
    # `swallowed_undef_control.asm` beside this file says in a comment that its
    # footer does NOT carry the line, and the first version of this check read
    # that sentence and reported the control as incomplete. Selfcheck case 11
    # caught it. Echoed source always carries a line-number and address prefix,
    # so requiring the phrase to start the line after whitespace alone
    # distinguishes the footer from any mention of it.
    if grep -qE '^[[:space:]]+Additional necessary passes not started' "$lst"; then
        echo INCOMPLETE
        return 0
    fi
    if grep -qE '^ +[0-9]+ passe?s?$' "$lst"; then
        echo complete
        return 0
    fi
    echo nofooter
}

# Which listing `asl -L` would have written for this argument vector. asl names
# the listing after the SOURCE FILE, beside it, extension replaced. Measured,
# not assumed: `asl -L -i . sub/x.asm` writes `sub/x.lst`, not `./x.lst`.
#
# AND IT TRUNCATES AT THE FIRST DOT IN THE PATH IT WAS GIVEN, NOT THE LAST.
# That is not a nicety. Measured on this build:
#
#     swal.asm                          ->  swal.lst          (as expected)
#     a.b/p.asm                         ->  a.lst             (in the PARENT)
#     ./p2.asm                          ->  .lst              (a hidden file)
#     /path/sigil/.claude/wt/x/p.asm    ->  /path/sigil/.lst
#
# So a caller inside a `.claude/worktrees/...` checkout that hands asl an
# ABSOLUTE source path gets no listing where it expects one and writes a stray
# `.lst` at the first dot-directory in the path, which in this workspace is the
# repository root, OUTSIDE the worktree. The untracked `.lst` and `.log` sitting
# at sigil's root are that. A runner in that shape then reads a listing that is
# missing or stale while its assembly looked entirely normal.
#
# `${src%%.*}` reproduces asl's rule exactly, which is the point: this function
# must agree with where the file went, not with where it ought to go. The last
# source-looking argument wins, and `-i .` / `-o out.p` do not match, so a
# blessed invocation resolves. Override with `ASL_LST` when a caller's argument
# vector is shaped so this cannot.
asl_lst_for() {
    local a src=""
    for a in "$@"; do
        case "$a" in
            -*) ;;
            *.asm|*.ASM|*.s|*.S|*.a68|*.z80|*.inc) src="$a" ;;
        esac
    done
    [ -n "$src" ] || return 1
    printf '%s.lst\n' "${src%%.*}"
}

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
    local asl_lst asl_diag
    asl_lst="${ASL_LST:-$(asl_lst_for "$@")}"
    asl_diag="$(asl_diag_state "$asl_lst")"
    case "$asl_diag" in
    INCOMPLETE)
        echo "ASL_DIAG=INCOMPLETE" >&2
        echo "  asl stopped its PASS LOOP on an error, so the diagnostics from" >&2
        echo "  the pass it refused to start were NEVER LOOKED FOR. Chiefly" >&2
        echo "  'symbol undefined': a forward reference is only judged on a" >&2
        echo "  later pass, so undefined symbols in this file are reported as" >&2
        echo "  ZERO here whether there are none or thirty. The error count in" >&2
        echo "  the footer counts what asl reached, not what is wrong." >&2
        echo "  THIS LINE MAKES THE GAP VISIBLE. IT DOES NOT FILL IT: fix the" >&2
        echo "  reported error and assemble again to learn what was hidden." >&2
        echo "  Listing: $asl_lst" >&2
        ;;
    complete)
        echo "ASL_DIAG=complete" >&2
        ;;
    nofooter)
        echo "ASL_DIAG=unknown ($asl_lst has no pass footer: the run died before" >&2
        echo "  writing one, so whether asl finished its pass loop is unknowable" >&2
        echo "  from the listing. Absence of the warning is NOT completeness.)" >&2
        ;;
    missing)
        if [ "$asl_rc" -eq 0 ]; then
            # No error occurred, and the pass loop only stops "due to errors",
            # so there is nothing a listing could have added here.
            echo "ASL_DIAG=complete (exit 0: no error, so no pass was aborted)" >&2
        else
            echo "ASL_DIAG=unknown (no listing at ${asl_lst:-<none derivable>};" >&2
            echo "  pass -L, or set ASL_LST, to learn whether asl finished its" >&2
            echo "  passes. If -L WAS passed: asl truncates the listing name at" >&2
            echo "  the FIRST dot in the source path it was given, so an absolute" >&2
            echo "  path through a dot-directory writes the listing elsewhere." >&2
            echo "  Hand it a relative path from the source's own directory.)" >&2
        fi
        ;;
    esac
    if [ "$asl_rc" -ne 0 ]; then
        echo "REFUSED: asl exited $asl_rc, so this run FAILED." >&2
        echo "  NO BYTE COLUMN FROM THIS RUN IS A SOURCE OF VALUES, including the" >&2
        echo "  lines that assembled: an error stops the pass loop, so every forward" >&2
        echo "  reference is left at its unresolved pass-1 placeholder and the" >&2
        echo "  listing prints them looking complete. See partial_failure.asm" >&2
        echo "  beside asl_ref.sh, where one bad line leaves 6702 reading 67FE." >&2
        echo "  Fix the source, do not quote the listing." >&2
    fi
    return "$asl_rc"
}
