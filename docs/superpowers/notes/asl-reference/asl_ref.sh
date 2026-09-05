#!/usr/bin/env bash
# asl_ref.sh — select the reference assembler, and REFUSE any other build.
#
# Source it, do not run it, and PROPAGATE ITS STATUS — these runners do not set
# `-e`, so a sourced guard that merely returns non-zero stops nothing:
#
#     . "$(dirname "$0")/../asl-reference/asl_ref.sh" || exit $?
#     # $ASLDIR is now a directory whose `asl` is the reference build,
#     # $ASL is that binary, and AS_MSGPATH is exported to match.
#
# Without the `|| exit $?` the runner assembles with a refused binary and the
# guard is decoration. `selfcheck.sh` case 4 is the one that catches dropping it.
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
