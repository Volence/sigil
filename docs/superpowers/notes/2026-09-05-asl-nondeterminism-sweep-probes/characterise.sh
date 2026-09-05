#!/usr/bin/env bash
# The asl value-instability, characterised — the minimal trigger, the regime that
# is SILENT, and which builds of asl carry it.
#
# ── WHAT THE DEFECT IS ───────────────────────────────────────────────────────
# Where a well-behaved build of AS substitutes 0 for an operand it could not
# resolve, one build substitutes an UNINITIALIZED MEMORY VALUE. `aslr.sh` beside
# this file is the proof of that mechanism: under `setarch -R` (address-space
# randomization off) the "varying" value collapses to a constant $5555, and with
# randomization on it is a fresh value essentially every run.
#
# ── IT IS BUILD-SPECIFIC, AND THE BANNER DOES NOT SEPARATE THE BUILDS ────────
# Four `asl` binaries sit in this workspace. All four print the same banner,
# `Macro Assembler 1.42 Beta [Bld 212]`, and they are NOT the same program:
#
#   61e672562465725a8c102288a7da9098   s1disasm, skdisasm, sonic_hack   STABLE, substitutes 0
#   0dee1f98e6480a4783d27ffd8b90896f   s2disasm                          UNSTABLE
#
# Every probe runner committed in this tree is pinned to the s1disasm path, and
# the three golden-vector generators take their binary from `ASL_BIN`. So the
# instability is reachable from this repo only by pointing `ASL_BIN` at the
# s2disasm build. The version string cannot tell an operator which one they have;
# the md5 can.
#
# ── THE MINIMAL TRIGGER IS NOT A `function`, A REGISTER, OR AN IMMEDIATE ─────
# The shape this defect was first reported under was `move.w #konst(a1),d0` — "a
# function call in an immediate whose argument is a register name". Each of those
# three attributes is separately unnecessary; `n_*.asm` are the peel:
#
#   n_reg_a1          #zz(a1)   zz undefined          UNSTABLE
#   n_nonreg_name     #zz(qq)   neither name defined  UNSTABLE  → not the register
#   n_number_arg      #zz(5)    numeric argument      UNSTABLE  → not the register
#   n_bare_paren_reg  #(a1)     no leading name       UNSTABLE  → not the function
#   u_bare            #zz       no parens at all      UNSTABLE  → not the call syntax
#   n_bare_paren_num  #(5)      everything resolves   stable
#   n_fn_num          #f(5)     f is a real function  stable
#   d_bare            #zz       zz defined            stable
#
# The minimal trigger is `move.w #zz,d0` with `zz` undefined. The common factor
# is an operand asl could not resolve, and the emitted placeholder for it.
#
# ── THE REGIME THAT MATTERS: SILENT AND UNSTABLE ─────────────────────────────
# The minimal form is LOUD — error #1010, exit 2 — so a tool that checks asl's
# exit status never banks it. The dangerous regime is the one that is quiet:
#
#   s_fn_defined_reg  #f(a1), f a defined `function`   exit 0, NO diagnostic, UNSTABLE
#   s_fn_body_uses_p  same, body references its param  exit 0, NO diagnostic, UNSTABLE
#   s_fline_shape     1+dsp(d3).w                      exit 0, NO diagnostic, UNSTABLE
#                                                       (alternates F83C/783C)
#
# Here asl peels the trailing `(register)` group as an addressing mode, so the
# name is left as a displacement that resolves to nothing, and the uninitialized
# value is emitted with a clean exit. THAT is the shape a golden-vector minting
# tool will accept and freeze, and it is why "the generator asserts asl exited 0"
# is NOT what protects the committed vectors.
#
# ── THE STABLE BUILD IS NOT A CORRECT BUILD ─────────────────────────────────
# Read the two halves of the table together. On the 61e672 build
# `s_fn_defined_reg` is exit 0, no diagnostic, and `303C 0000` — SILENTLY WRONG,
# just deterministically so. The defect both builds share is that `#f(<reg>)` is
# accepted at all; they differ only in the placeholder, 0 against uninitialized
# memory. So "our pinned asl is stable here" is a statement about
# REPRODUCIBILITY and about nothing else: a vector minted from this shape on the
# good build is a reproducible wrong answer, which a stability sweep cannot see
# and no number of runs will surface.
#
# `s_fline_shape` is the same story with a second twist: the `F83C` F-line word
# reported for `1+dsp(d3).w` is a property of the 0dee1f98 build, where it also
# ALTERNATES with `783C`; the 61e672 build emits a stable `343C 1234` instead.
set -uo pipefail
declare -a ASL_DIRS=("${ASL_DIRS[@]:-}")
[[ -n ${ASL_DIRS[0]:-} ]] || ASL_DIRS=()
HERE="$(cd "$(dirname "$0")" && pwd)"
N="${1:-8}"
WORK="${TMPDIR:-/tmp}/asl_characterise.$$"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT
cp "$HERE"/*.asm "$WORK"/

PROBES=(u_bare u_expr u_paren u_dcw u_fwd d_bare
        n_reg_a1 n_nonreg_name n_defined_nonreg n_number_arg
        n_bare_paren_reg n_bare_paren_num n_dcw_ctx n_equ_ctx n_dst_operand
        n_fn_reg n_fn_num n_z80_reg
        s_fn_defined_reg s_fn_body_uses_p s_fline_shape s_undefined)

# Explicit array — never a `${arr[@]:-a b}` default, which is one word under zsh
# and two under bash, so the "both binaries" arm silently becomes one.
if [[ ${#ASL_DIRS[@]} -eq 0 ]]; then
    ASL_DIRS=(/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64
              /home/volence/sonic_hacks/s2disasm/build_tools/Linux-x86_64
              /home/volence/sonic_hacks/skdisasm/build_tools/Linux-x86_64
              /home/volence/sonic_hacks/sonic_hack/tools/as)
fi
echo "# asl builds under test: ${#ASL_DIRS[@]}"
for ASLDIR in "${ASL_DIRS[@]}"; do
    [[ -x $ASLDIR/asl ]] || { echo "# SKIP $ASLDIR (no asl) — reported, not silently dropped"; continue; }
    echo "# asl $ASLDIR/asl md5 $(md5sum "$ASLDIR/asl" | cut -d' ' -f1)  N=$N"
    for p in "${PROBES[@]}"; do
        [[ -f $WORK/$p.asm ]] || { printf '  %-8s %-18s (no such probe)\n' MISSING "$p"; continue; }
        vals=""; rc=""; diag=""
        for i in $(seq 1 "$N"); do
            rm -f "$WORK/$p.p" "$WORK/$p.lst"
            out="$(cd "$WORK" && AS_MSGPATH="$ASLDIR" timeout 60 "$ASLDIR/asl" -xx -n -q -A -L -U -i "$WORK" "$p.asm" 2>&1)"
            rc=$?
            vals="$vals $(grep -oE ': [0-9A-F]{4}( [0-9A-F]{4})?' "$WORK/$p.lst" 2>/dev/null | tr -d ': ' | tr '\n' '/')"
            [[ $i -eq 1 ]] && diag="$(printf '%s' "$out" | grep -oE 'error #[0-9]+|warning #[0-9]+' | head -1)"
        done
        u="$(printf '%s\n' $vals | sort -u | wc -l)"
        tag=STABLE; [[ $u -eq 1 ]] || tag=UNSTABLE
        printf '  %-8s %-18s exit=%-3s %-12s %s\n' "$tag" "$p" "$rc" "${diag:-silent}" "$vals"
    done
done
