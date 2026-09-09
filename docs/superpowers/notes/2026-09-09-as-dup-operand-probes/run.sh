#!/bin/bash
# Differential probe runner for AS's `[count]value` duplicate-operand syntax.
# usage: run.sh <file.asm>  -- assembles with S1's own flags and prints the listing.
#
# The assembler is selected by MD5, not by path and not by version banner: several
# `asl` binaries in this workspace print the same banner and are not the same
# program, and one of them answers refused operands from uninitialized memory.
# `asl_ref.sh` refuses anything but the reference build, and `asl_run` refuses a
# non-zero exit out loud: a run carrying ANY error is not a source of values for
# the lines that did assemble. `|| exit $?` is load-bearing because `set -u` is
# not `set -e`.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
F="$1"; B="${F%.asm}"
cd "$(dirname "$F")" || exit 9
rm -f "$B.lst" "$B.p"
asl_run -xx -n -q -A -L -U -i . "$(basename "$F")"
st=$?
echo "asl_run status=$st"
asl_diag_state "$(basename "$B").lst"
grep -v '^ *$' "$(basename "$B").lst"
exit $st
