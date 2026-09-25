#!/usr/bin/env bash
# run_probes.sh <sigil-binary> <out.tsv> <work-dir>: every probes/*.asm beside this
# script, under the pinned asl (asl_run) and under the given sigil. One row per probe:
#   name asl_rc asl_first_error asl_bytes sigil_rc sigil_first_error sigil_bytes verdict asl_warnings
# asl bytes come from p2bin over asl's .p ONLY when asl exited 0; otherwise "-".
# Bytes are read from $1200 on, the probes' org. <work-dir> must be outside the tree.
set -u
S="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$S/../../../.." && pwd)"
SIG="$1"; OUT="$2"; WORK="$3"
. "$REPO/docs/superpowers/notes/asl-reference/asl_ref.sh" || exit $?
P2BIN=$ASLDIR/p2bin
mkdir -p "$WORK"
: > "$OUT"
for f in "$S"/probes/*.asm; do
  n=$(basename "$f" .asm)
  d="$WORK/$n"; rm -rf "$d"; mkdir -p "$d"; cp "$f" "$d/probe.asm"
  ( cd "$d" && timeout 30 bash -c '. "$0" && asl_run -xx -n -q -A -L -U -i . probe.asm' "$REPO/docs/superpowers/notes/asl-reference/asl_ref.sh" ) > "$d/asl.out" 2> "$d/asl.err"
  arc=$?
  amsg=$(sed -n 's/^.*: \(error\|fatal error\) #\([0-9]\+\): \(.*\)$/#\2 \3/p' "$d/asl.err" | head -1 | sed 's/[[:space:]]*$//')
  [ "$arc" -ne 0 ] && [ -z "$amsg" ] && amsg="(no numbered diagnostic)"
  awarn=$(grep -c 'warning #' "$d/asl.err")
  abytes=-
  if [ "$arc" -eq 0 ]; then
    ( cd "$d" && timeout 30 "$P2BIN" probe.p asl.bin -p=0 ) > /dev/null 2>&1
    abytes=$(xxd -s 0x1200 -p -c 4096 "$d/asl.bin" 2>/dev/null | tr -d '\n')
    [ -n "$abytes" ] || abytes="(empty)"
  fi
  ( cd "$d" && timeout 30 "$SIG" probe.asm -o sigil.bin ) > "$d/sigil.out" 2>&1
  src=$?
  smsg=$(grep -m1 -iE 'error' "$d/sigil.out" | sed 's/[[:space:]]*$//')
  [ "$src" -ne 0 ] && [ -z "$smsg" ] && smsg=$(head -1 "$d/sigil.out")
  sbytes=-
  if [ "$src" -eq 0 ]; then
    sbytes=$(xxd -s 0x1200 -p -c 4096 "$d/sigil.bin" 2>/dev/null | tr -d '\n')
    [ -n "$sbytes" ] || sbytes="(empty)"
  fi
  if [ "$arc" -eq 0 ] && [ "$src" -eq 0 ]; then
    if [ "$abytes" = "$sbytes" ]; then v=same; else v=BYTES-DIFFER; fi
  elif [ "$arc" -ne 0 ] && [ "$src" -ne 0 ]; then v=both-refuse
  elif [ "$arc" -ne 0 ]; then v=OVER-ACCEPT
  else v=OVER-REFUSE; fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$n" "$arc" "$amsg" "$abytes" "$src" "$smsg" "$sbytes" "$v" "$awarn" >> "$OUT"
done
echo "RUN_PROBES_END $(wc -l < "$OUT") rows"
