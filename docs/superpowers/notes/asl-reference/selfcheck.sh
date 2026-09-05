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
# ── WHAT MUST FAIL ───────────────────────────────────────────────────────────
# This script MUST FAIL if: the guard accepts the varying build; the guard
# accepts a missing binary; the guard's literal digest is edited to match
# whatever is on disk; a runner drops the `|| exit $?`; or the guard is stubbed
# out. If any of those leaves this green, the guard is decoration and the
# selfcheck is too.
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
if printf '%s' "$msg" | grep -q "$VARYING_MD5" && printf '%s' "$msg" | grep -q "$STABLE_MD5"; then
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
STUB="$(mktemp)"; trap 'rm -f "$STUB"' EXIT
sed 's/^if \[ "\$ASL_REF_GOT" != "\$ASL_REF_MD5" \]; then$/if false; then/' "$GUARD" > "$STUB"
if ! grep -q '^if false; then$' "$STUB"; then
    bad "the stub did not apply — case 5 would have passed by running the ORIGINAL guard, \
which is indistinguishable from a working stub"
elif [ "$(probe "$VARYING_DIR" "$STUB")" = 0 ]; then
    ok "stubbed guard accepts the varying build, so case 2 is measuring the comparison"
else
    bad "stubbed guard still refused — case 2 is passing for some reason other than the md5 test"
fi

echo
echo "pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
