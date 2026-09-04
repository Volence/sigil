#!/bin/bash
# usage: run.sh <asl-path> <file.asm> <outprefix>
ASL="$1"; SRC="$2"; OUT="$3"
rm -f "$OUT.lst" "$OUT.p" "$OUT.log"
timeout 60 "$ASL" -xx -n -q -A -L -U -i . -olist "$OUT.lst" -o "$OUT.p" "$SRC" > "$OUT.log" 2>&1
echo "asl_exit=$?"
