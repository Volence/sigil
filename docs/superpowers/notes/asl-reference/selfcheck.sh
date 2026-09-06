#!/usr/bin/env bash
# selfcheck.sh — PROVE THE MD5 GUARD FIRES.
#
#     ./selfcheck.sh          # exit 0 = every case as required
#
# ── WHY THIS EXISTS ──────────────────────────────────────────────────────────
# `asl_ref.sh` refuses any `asl` but the reference build. A guard that has never
# been SEEN to refuse is indistinguishable from no guard at all: it reads green
# forever whether or not it works, and the thing it is protecting against —
# a runner that verifies nothing while claiming to identify its instrument —
# is exactly what it would have become.
#
# So case 2 is the load-bearing one: pointed at the VARYING build, the guard must
# come back non-zero. Cases 1, 3, 4 and 5 fence it in from the other sides.
#
# Case 5 is this file's own honesty check. It stubs the guard's comparison to
# always accept and requires case 2 to go RED under the stub — because a
# selfcheck that stays green when the guard is disabled is measuring nothing,
# which is the same defect one level up.
#
# ── CASES 6 TO 9: THE RUN, NOT THE BUILD ─────────────────────────────────────
# Cases 0 to 5 all ask WHICH PROGRAM RAN. Cases 6 to 9 ask WHETHER ITS ANSWERS
# MEAN ANYTHING, which is `asl_run`, and they are built around the shape that is
# actually dangerous rather than the easy one.
#
# THE EASY CASE IS A FILE THAT DOES NOT ASSEMBLE. Nobody quotes a listing that
# is not there. The dangerous case is `partial_failure.asm` beside this file: it
# MOSTLY assembles, asl exits 2, and the listing carries a full byte column for
# every line that succeeded, one of which the error silently changed. Case 7
# measures that property directly rather than assuming it, by assembling the
# same file with the one bad line deleted and requiring the value to MOVE. If it
# ever stops moving, case 7 fails and says the fixture is no longer the shape
# this is here to cover.
#
# Case 6 is the not-always-red side: `asl_run` must ACCEPT a clean assembly
# without the banner. A check that fires on correct input trains people to
# weaken it, so it is fenced from that direction too.
#
# Case 8 is the load-bearing new one: `asl_run` REFUSES the partial failure.
# Case 9 is its honesty check, and it is stubbed at a different place from case
# 5 on purpose: case 5 stubs the DIGEST comparison, case 9 stubs `asl_run`'s
# status propagation, because those are two separate checks and a stub of one
# says nothing about the other.
#
# ── WHAT MUST FAIL ───────────────────────────────────────────────────────────
# This script MUST FAIL if: the guard accepts the varying build; the guard
# accepts a missing binary; the guard's literal digest is edited to match
# whatever is on disk; a runner drops the `|| exit $?`; the guard is stubbed
# out; `asl_run` returns zero on a failed assembly; `asl_run` reports a clean
# assembly as failed; the fixture stops being a PARTIAL failure; or `asl_run`'s
# status propagation is stubbed away. If any of those leaves this green, the
# guard is decoration and the selfcheck is too.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
GUARD="$HERE/asl_ref.sh"

# Written out, not read off any binary: these two are the STATED identities of
# the two builds, and the whole point is that the banner cannot distinguish them.
STABLE_DIR=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64
STABLE_MD5=61e672562465725a8c102288a7da9098
VARYING_DIR=/home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64
VARYING_MD5=0dee1f98e6480a4783d27ffd8b90896f

pass=0; fail=0
ok()   { echo "  PASS  $1"; pass=$((pass+1)); }
bad()  { echo "  FAIL  $1"; fail=$((fail+1)); }

# One EXIT trap for every temporary this file makes. A second `trap ... EXIT`
# REPLACES the first rather than adding to it, so the cases below register their
# temporaries here instead of each installing a trap of its own.
STUB=""; ASLSTUB=""; SCRATCH=""
cleanup() {
    [ -n "$STUB" ]    && rm -f "$STUB"
    [ -n "$ASLSTUB" ] && rm -f "$ASLSTUB"
    [ -n "$SCRATCH" ] && rm -rf "$SCRATCH"
    return 0
}
trap cleanup EXIT

# Run the guard in a child shell with $ASLDIR set to $1; echo its exit status.
probe() {
    ( ASLDIR="$1"; . "${2:-$GUARD}" >/dev/null 2>&1 || exit $?; exit 0 )
    echo $?
}
probe_msg() {
    ( ASLDIR="$1"; . "$GUARD" 2>&1 >/dev/null || true )
}

echo "--- case 0: the two builds print the SAME banner, which is why md5 is the check"
# asl with no arguments prints its banner and exits; there is no `--version`.
b_stable="$(AS_MSGPATH="$STABLE_DIR" timeout 20 "$STABLE_DIR/asl" 2>&1 | grep -oE 'Macro Assembler [0-9.]+ Beta \[Bld [0-9]+\]' | head -1)"
b_varying="$(AS_MSGPATH="$VARYING_DIR" timeout 20 "$VARYING_DIR/asl" 2>&1 | grep -oE 'Macro Assembler [0-9.]+ Beta \[Bld [0-9]+\]' | head -1)"
if [ -n "$b_stable" ] && [ "$b_stable" = "$b_varying" ]; then
    ok "both report '$b_stable' — a version check cannot discriminate"
else
    bad "expected identical non-empty banners, got stable='$b_stable' varying='$b_varying'"
fi

echo "--- case 1: the reference build is ACCEPTED"
got="$(md5sum "$STABLE_DIR/asl" | cut -d' ' -f1)"
if [ "$got" != "$STABLE_MD5" ]; then
    bad "the binary at $STABLE_DIR is md5 $got, not the stated reference $STABLE_MD5 — \
this selfcheck's premise is gone, fix the pin rather than the digest"
elif [ "$(probe "$STABLE_DIR")" = 0 ]; then
    ok "guard exits 0 on md5 $STABLE_MD5"
else
    bad "guard REFUSED the reference build"
fi

echo "--- case 2: THE VARYING BUILD IS REFUSED (the load-bearing case)"
got="$(md5sum "$VARYING_DIR/asl" | cut -d' ' -f1)"
if [ "$got" != "$VARYING_MD5" ]; then
    bad "the binary at $VARYING_DIR is md5 $got, not the stated varying build $VARYING_MD5 — \
this case is no longer pointed at the thing it exists to refuse"
elif [ "$(probe "$VARYING_DIR")" != 0 ]; then
    ok "guard refuses md5 $VARYING_MD5, the build whose refused operands vary run to run"
else
    bad "GUARD ACCEPTED THE VARYING BUILD — it is decoration"
fi

echo "--- case 3: the refusal NAMES the digest it saw, so the message is diagnosable"
msg="$(probe_msg "$VARYING_DIR")"
# THE TEST IS ASKED IN THE SHELL, WITH NO PIPE, and that is load-bearing. This was
# `printf '%s' "$msg" | grep -q "$VARYING_MD5" && …`, which is wrong under this
# file's own `set -o pipefail`: `grep -q` exits the moment it MATCHES, `printf` is
# then killed by SIGPIPE, and `pipefail` hands the pipeline back 141 — so a match
# reads as a NON-match and case 3 fails on a message that was correct. Whether
# printf's write lands before grep exits is a scheduling race, so it fires only
# sometimes, and only in the direction that reports a false FAIL.
if [[ $msg == *"$VARYING_MD5"* && $msg == *"$STABLE_MD5"* ]]; then
    ok "message carries both the wanted and the seen digest"
else
    bad "refusal message does not name both digests: [$msg]"
fi

echo "--- case 4: a MISSING binary is refused, not silently skipped"
if [ "$(probe "$HERE/no-such-dir")" != 0 ]; then
    ok "guard refuses a directory with no asl in it"
else
    bad "guard accepted a nonexistent binary"
fi

echo "--- case 5: with the comparison STUBBED TO ACCEPT, case 2 must go RED"
STUB="$(mktemp)"
sed 's/^if \[ "\$ASL_REF_GOT" != "\$ASL_REF_MD5" \]; then$/if false; then/' "$GUARD" > "$STUB"
if ! grep -q '^if false; then$' "$STUB"; then
    bad "the stub did not apply — case 5 would have passed by running the ORIGINAL guard, \
which is indistinguishable from a working stub"
elif [ "$(probe "$VARYING_DIR" "$STUB")" = 0 ]; then
    ok "stubbed guard accepts the varying build, so case 2 is measuring the comparison"
else
    bad "stubbed guard still refused — case 2 is passing for some reason other than the md5 test"
fi


# ── CASES 6 TO 9: THE RUN ────────────────────────────────────────────────────
SCRATCH="$(mktemp -d)"
FIXTURE="$HERE/partial_failure.asm"
if [ ! -f "$FIXTURE" ]; then
    bad "no partial_failure.asm beside this file: cases 6 to 9 cannot run, and \
a missing fixture is not a pass"
else
cp "$FIXTURE" "$SCRATCH/pf.asm"
# The control is the fixture MINUS its one bad instruction line, and nothing
# else. It is derived from the subject on purpose: the expectations below are
# written-out literals, so deriving the control does not move them.
sed '/^[[:blank:]]*bra\.s/d' "$FIXTURE" > "$SCRATCH/ctl.asm"

# The values this build assembles `beq.s +` to in the two files. Written out,
# not read off either listing: an expectation taken from the subject moves with
# the subject and can never come out red.
BEQ_BROKEN=67FE     # branch to ITSELF, the corrupted value
BEQ_CORRECT=6702    # the correct forward branch over the `nop`

# Assemble $1 through `asl_run`, guard at $2 (default $GUARD). Line 1 is the
# return status; the rest is asl_run's stderr. NO PIPE ANYWHERE, deliberately:
# an early-exiting reader under this file's `pipefail` is exactly the fault
# case 3's comment records.
run_probe() {
    (
        cd "$SCRATCH" || exit 90
        export USEANSI=n
        . "${2:-$GUARD}" >/dev/null 2>&1 || exit $?
        err="$(asl_run -xx -n -q -A -L -U -i "$SCRATCH" "$1" 2>&1 >/dev/null)"
        rc=$?
        printf '%s\n%s\n' "$rc" "$err"
    )
}
# The same assembly through the UNBLESSED path, so case 7 measures what a caller
# who never adopted `asl_run` actually sees.
raw_probe() {
    (
        cd "$SCRATCH" || exit 90
        export USEANSI=n
        . "$GUARD" >/dev/null 2>&1 || exit $?
        "$ASL" -xx -n -q -A -L -U -i "$SCRATCH" "$1" >/dev/null 2>&1
        exit $?
    )
}
# The byte column asl printed for the macro-expanded `beq.s`. Read from the file
# directly rather than through a pipe, for the same reason as above.
beq_bytes() { awk '/beq\.s/ && $4 ~ /^[0-9A-F]+$/ { print $4; exit }' "$1"; }

echo "--- case 6: asl_run ACCEPTS a clean assembly, with no banner"
out="$(run_probe ctl.asm)"; rc="${out%%$'\n'*}"; msg="${out#*$'\n'}"
if [ "$rc" != 0 ]; then
    bad "asl_run returned $rc on a file that assembles cleanly: a check that \
fires on correct input is the shape people weaken: [$msg]"
elif [[ $msg == *REFUSED* ]]; then
    bad "asl_run printed REFUSED on a clean assembly: [$msg]"
elif [ "$(beq_bytes "$SCRATCH/ctl.lst")" != "$BEQ_CORRECT" ]; then
    bad "the control assembles beq.s to $(beq_bytes "$SCRATCH/ctl.lst"), not \
$BEQ_CORRECT: this build no longer agrees with the pinned values, fix the pin \
rather than the expectation"
else
    ok "clean run: exit 0, no banner, beq.s = $BEQ_CORRECT"
fi

echo "--- case 7: THE FIXTURE IS A PARTIAL FAILURE, not a file that fails to assemble"
raw_probe pf.asm; raw_rc=$?
lst="$(cat "$SCRATCH/pf.lst" 2>/dev/null || true)"
got_broken="$(beq_bytes "$SCRATCH/pf.lst" 2>/dev/null || true)"
if [ "$raw_rc" = 0 ]; then
    bad "asl exited 0 on the fixture: it is supposed to carry an error"
elif [ -z "$lst" ]; then
    bad "no listing at all: the fixture became the EASY case (nothing to quote), \
which is not the shape cases 8 and 9 exist to cover"
elif [[ $lst != *4E75* ]]; then
    bad "the listing has no byte column past the error (no 4E75 for the rts): \
the fixture stopped being the dangerous shape"
elif [ "$got_broken" != "$BEQ_BROKEN" ]; then
    bad "beq.s came back $got_broken, not the corrupted $BEQ_BROKEN: the fixture \
no longer demonstrates one error changing another line's value"
elif [ "$BEQ_BROKEN" = "$BEQ_CORRECT" ]; then
    bad "the broken and correct values are the same literal, so this case cannot fail"
else
    ok "exit $raw_rc, full byte column (4E75 present), and beq.s reads \
$BEQ_BROKEN here against $BEQ_CORRECT in the control: one error, another line's \
value changed, nothing announcing it"
fi

echo "--- case 8: asl_run REFUSES the partial failure (the load-bearing new case)"
out="$(run_probe pf.asm)"; rc="${out%%$'\n'*}"; msg="${out#*$'\n'}"
if [ "$rc" = 0 ]; then
    bad "ASL_RUN RETURNED 0 ON A FAILED ASSEMBLY: the exit check is decoration"
elif [[ $msg != *REFUSED* ]]; then
    bad "asl_run returned $rc but printed no refusal: [$msg]"
elif [[ $msg != *"ASL_EXIT=$rc"* ]]; then
    bad "the refusal does not name the status it saw, so it is not diagnosable: [$msg]"
else
    ok "asl_run returns $rc and refuses out loud, naming ASL_EXIT=$rc"
fi

echo "--- case 9: with asl_run's STATUS PROPAGATION stubbed, case 8 must go RED"
# Stubbed at a different point from case 5 on purpose: case 5 disables the DIGEST
# comparison, this disables the EXIT check, and a stub of one proves nothing
# about the other.
ASLSTUB="$(mktemp)"
sed 's/^    return "\$asl_rc"$/    return 0/' "$GUARD" > "$ASLSTUB"
# THE STUB MUST HAVE CHANGED SOMETHING, and "the output contains `return 0`" does
# not establish that: it is also true of a guard that ALREADY said `return 0`,
# which is the very defect this case exists to detect. So both halves are asked:
# the guard carries the line before, and the stub does not carry it after.
if ! grep -q '^    return "\$asl_rc"$' "$GUARD"; then
    bad "the guard has no 'return \$asl_rc' line to stub: either asl_run's status \
propagation is already gone (which case 8 should be reporting) or the line was \
renamed and this case is stubbing nothing"
elif grep -q '^    return "\$asl_rc"$' "$ASLSTUB"; then
    bad "the stub did not apply: case 9 would have passed by running the ORIGINAL \
guard, which is indistinguishable from a working stub"
elif ! grep -q '^    return 0$' "$ASLSTUB"; then
    bad "the stub removed the line without substituting 'return 0' for it, so the \
stubbed guard is not the program this case means to run"
else
    out="$(run_probe pf.asm "$ASLSTUB")"; rc="${out%%$'\n'*}"
    if [ "$rc" = 0 ]; then
        ok "stubbed asl_run returns 0 on the failed assembly, so case 8 is \
measuring the exit check and not something else"
    else
        bad "stubbed asl_run still returned $rc: case 8 is passing for some \
reason other than the status test"
    fi
fi

# ── CASES 10 TO 13: THE PASS LOOP ────────────────────────────────────────────
# A DIFFERENT DEFECT SHAPE FROM CASES 6 TO 9, and the reason cases 8 and 9 do
# not cover it. There the run failed and `asl_run` refused a non-zero exit.
# Here the exit is non-zero too - legitimately, the run really did fail - and
# the defect is that it failed for reason X while reporting NOTHING about
# reason Y. A check keyed to "did it fail" cannot see that, because both arms
# of the pair below fail.
#
# `swallowed_undef.asm` carries one unrelated error above three undefined
# symbols; `swallowed_undef_control.asm` is the same file without that one line.
# Case 10 measures the pair with no check involved at all, so cases 11 to 13
# are known to be reporting on a real difference rather than on themselves.
SWAL="$HERE/swallowed_undef.asm"
SWCTL="$HERE/swallowed_undef_control.asm"
if [ ! -f "$SWAL" ] || [ ! -f "$SWCTL" ]; then
    bad "swallowed_undef.asm or swallowed_undef_control.asm is missing beside \
this file: cases 10 to 13 cannot run, and a missing fixture is not a pass"
else
cp "$SWAL" "$SCRATCH/swal.asm"
cp "$SWCTL" "$SCRATCH/swctl.asm"

# Written-out expectations, not read off either listing.
UNDEF_WHEN_LOOKED=3      # what the control reports
UNDEF_WHEN_SWALLOWED=0   # what the subject reports, having the same three

undef_count() { grep -c 'error #1010' "$1" 2>/dev/null || true; }

echo "--- case 10: THE PAIR DISCRIMINATES, with no check of ours in the loop"
raw_probe swal.asm;  swal_rc=$?
raw_probe swctl.asm; swctl_rc=$?
got_swal="$(undef_count "$SCRATCH/swal.lst")"
got_swctl="$(undef_count "$SCRATCH/swctl.lst")"
if [ "$UNDEF_WHEN_LOOKED" = "$UNDEF_WHEN_SWALLOWED" ]; then
    bad "the two expectations are the same literal, so this case cannot fail"
elif [ "$swal_rc" = 0 ] || [ "$swctl_rc" = 0 ]; then
    bad "one arm exited 0 (subject $swal_rc, control $swctl_rc): the point of \
this pair is that BOTH fail, and an arm that passes makes the exit status \
sufficient after all"
elif [ "$got_swctl" != "$UNDEF_WHEN_LOOKED" ]; then
    bad "the control reported $got_swctl undefined symbols, not \
$UNDEF_WHEN_LOOKED: the fixture no longer contains the class this pair is \
about, so a zero from the subject would mean nothing"
elif [ "$got_swal" != "$UNDEF_WHEN_SWALLOWED" ]; then
    bad "the subject reported $got_swal undefined symbols, not \
$UNDEF_WHEN_SWALLOWED: this build no longer swallows them, so cases 11 to 13 \
are checking for a condition that does not arise"
else
    ok "both arms exit non-zero (subject $swal_rc, control $swctl_rc), the \
control reports $got_swctl x #1010 and the subject reports $got_swal x #1010 \
for the SAME three symbols: a failure that looked, and a failure that did not"
fi

echo "--- case 11: asl_run CALLS IT (INCOMPLETE on the subject, complete on the control)"
out="$(run_probe swal.asm)";  msg_swal="${out#*$'\n'}"
out="$(run_probe swctl.asm)"; rc_swctl="${out%%$'\n'*}"; msg_swctl="${out#*$'\n'}"
if [[ $msg_swal != *"ASL_DIAG=INCOMPLETE"* ]]; then
    bad "asl_run said nothing about the swallowed pass on the subject: [$msg_swal]"
elif [[ $msg_swal != *"NEVER LOOKED FOR"* ]]; then
    bad "the INCOMPLETE report does not say what it means, so it is not \
actionable: [$msg_swal]"
elif [[ $msg_swctl == *"ASL_DIAG=INCOMPLETE"* ]]; then
    bad "asl_run called the CONTROL incomplete. It failed with $rc_swctl and \
three real errors, and its diagnostics are whole. A detector that reds every \
failing run is the always-red shape, and this tree is full of probes that are \
supposed to fail: [$msg_swctl]"
elif [[ $msg_swctl != *"ASL_DIAG=complete"* ]]; then
    bad "asl_run reported no diagnostic state at all for the control: [$msg_swctl]"
else
    ok "subject INCOMPLETE, control complete, and the control still failed with \
$rc_swctl: the check separates the two failures rather than firing on both"
fi

echo "--- case 12: the state NEVER changes what asl_run returns"
# Deliberate, and worth a case of its own: several probes in this tree read a
# non-zero exit as their answer, and a diagnostic-completeness report that moved
# the status would break them silently.
out="$(run_probe swal.asm)"; rc_swal="${out%%$'\n'*}"
raw_probe swal.asm; raw_swal_rc=$?
if [ "$rc_swal" != "$raw_swal_rc" ]; then
    bad "asl_run returned $rc_swal where raw asl returned $raw_swal_rc: the \
completeness report is not supposed to touch the status"
elif [ "$rc_swal" = 0 ]; then
    bad "asl_run returned 0 on the subject, so this case is comparing two \
values that are both wrong"
else
    ok "asl_run returns $rc_swal, the same status raw asl returned, with the \
INCOMPLETE report printed beside it and not instead of it"
fi

echo "--- case 13: with the footer literal STUBBED, case 11 must go RED"
# Stubbed at a third point, distinct from case 5 (the digest comparison) and
# case 9 (the exit propagation): this disables only the footer match, so a stub
# of either of the others proves nothing about this one.
DIAGSTUB="$(mktemp)"
DIAG_LIT='Additional necessary passes not started'
sed "s/'\^\[\[:space:\]\]+$DIAG_LIT'/'ZZ_NO_SUCH_FOOTER_LINE_ZZ'/" "$GUARD" > "$DIAGSTUB"
if ! grep -qF "grep -qE '^[[:space:]]+$DIAG_LIT'" "$GUARD"; then
    bad "the guard does not match on '$DIAG_LIT': either the pass-loop check is \
already gone (which case 11 should be reporting) or it was rewritten and this \
case is stubbing nothing"
elif grep -qF "grep -qE '^[[:space:]]+$DIAG_LIT'" "$DIAGSTUB"; then
    bad "the stub did not apply: case 13 would have passed by running the \
ORIGINAL guard, which is indistinguishable from a working stub"
elif ! grep -q 'ZZ_NO_SUCH_FOOTER_LINE_ZZ' "$DIAGSTUB"; then
    bad "the stub removed the literal without substituting the unmatchable one, \
so the stubbed guard is not the program this case means to run"
else
    out="$(run_probe swal.asm "$DIAGSTUB")"; msg="${out#*$'\n'}"
    if [[ $msg == *"ASL_DIAG=INCOMPLETE"* ]]; then
        bad "the stubbed guard STILL called the subject incomplete: case 11 is \
passing for some reason other than the footer match"
    elif [[ $msg != *"ASL_DIAG=complete"* ]]; then
        bad "the stubbed guard reported neither state: [$msg]"
    else
        ok "stubbed guard calls the swallowing run 'complete', so case 11 is \
measuring the footer match and not something else"
    fi
fi

echo "--- case 14: asl_lst_for agrees with WHERE ASL ACTUALLY WROTE, dots and all"
# The whole INCOMPLETE check reads a file, so a derivation that names the wrong
# file reports `unknown` on a swallowing run and looks like a clean absence.
# asl truncates the listing name at the FIRST dot in the path it is handed, so a
# directory with a dot in it moves the listing into the PARENT. This case does
# not assume that: it assembles into such a directory and asks the filesystem.
DOTDIR="$SCRATCH/d.d"
mkdir -p "$DOTDIR"
cp "$SWAL" "$DOTDIR/p.asm"
rm -f "$SCRATCH"/d.lst "$DOTDIR"/p.lst
(
    cd "$SCRATCH" || exit 90
    export USEANSI=n
    . "$GUARD" >/dev/null 2>&1 || exit $?
    "$ASL" -xx -n -q -A -L -U -i . d.d/p.asm >/dev/null 2>&1
)
derived="$(
    . "$GUARD" >/dev/null 2>&1
    cd "$SCRATCH" || exit 90
    asl_lst_for -xx -n -q -A -L -U -i . d.d/p.asm
)"
actual="$(cd "$SCRATCH" && find . -name '*.lst' -newer "$DOTDIR/p.asm" | sed 's|^\./||' | head -1)"
if [ -z "$actual" ]; then
    bad "no listing was written anywhere under the scratch tree, so this case \
has nothing to compare against"
elif [ -z "$derived" ]; then
    bad "asl_lst_for derived nothing for a blessed argument vector, so asl_run \
would report 'unknown' on every run shaped like this"
elif [ "$derived" != "$actual" ]; then
    bad "asl_lst_for says '$derived' and asl actually wrote '$actual': the \
INCOMPLETE check would be reading the wrong file, and a missing file reports \
as 'unknown' rather than as an error"
else
    ok "asl wrote '$actual' for source d.d/p.asm and asl_lst_for names the same \
file: the derivation follows asl's first-dot rule rather than the obvious one"
fi
fi
fi

echo
echo "pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
