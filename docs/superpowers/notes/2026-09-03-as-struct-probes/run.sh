#!/usr/bin/env bash
# Run S1's asl on a probe file; print exit, listing body, and any errors.
ASL=/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl
d=$(dirname "$1"); f=$(basename "$1" .asm)
rm -f "$d/$f.p" "$d/$f.lst" "$d/$f.log"
( cd "$d" && "$ASL" -xx -n -q -A -L -U -E -i . "$f.asm" >/dev/null 2>&1 )
echo "asl exit=$?"
[[ -f $d/$f.log ]] && { echo "--- log ---"; cat "$d/$f.log"; }
[[ -f $d/$f.lst ]] && { echo "--- lst ---"; cat "$d/$f.lst"; }
