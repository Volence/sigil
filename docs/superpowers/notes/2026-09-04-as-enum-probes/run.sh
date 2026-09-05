#!/bin/bash
# Differential probe for AS's enum/nextenum/enumconf.
# usage: run.sh <file.asm>   -- runs the reference asl with S1's own flags, prints the listing.
#
# The assembler is selected by MD5, not by path and not by version banner: four
# `asl` binaries in this workspace print the same banner and are not the same
# program, and one of them answers refused operands from uninitialized memory.
# `asl_ref.sh` refuses anything but the reference build; `|| exit $?` is
# load-bearing because `set -u` is not `set -e`.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
A="$ASL"
F="$1"; B="${F%.asm}"
cd "$(dirname "$F")" || exit 9
rm -f "$B.lst" "$B.p"
"$A" -xx -n -q -A -L -U -i . "$(basename "$F")" 2>&1
echo "asl exit=$?"
grep -v '^ *$' "$B.lst" | sed -n '1,40p'
