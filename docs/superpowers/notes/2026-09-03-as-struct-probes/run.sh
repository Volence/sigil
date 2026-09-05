#!/usr/bin/env bash
# Run the reference asl on a probe file; print exit, listing body, and any errors.
#
# The assembler is selected by MD5, not by path and not by version banner: four
# `asl` binaries in this workspace print the same banner and are not the same
# program, and one of them answers refused operands from uninitialized memory.
# `asl_ref.sh` refuses anything but the reference build; `|| exit $?` is
# load-bearing because this script does not set `-e`.
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/../asl-reference/asl_ref.sh" || exit $?
d=$(dirname "$1"); f=$(basename "$1" .asm)
rm -f "$d/$f.p" "$d/$f.lst" "$d/$f.log"
( cd "$d" && "$ASL" -xx -n -q -A -L -U -E -i . "$f.asm" >/dev/null 2>&1 )
echo "asl exit=$?"
[[ -f $d/$f.log ]] && { echo "--- log ---"; cat "$d/$f.log"; }
[[ -f $d/$f.lst ]] && { echo "--- lst ---"; cat "$d/$f.lst"; }
