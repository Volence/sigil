#!/bin/bash
# asl_probe.sh <asl-binary-dir> ; reads snippets one per line on stdin,
# prints "<snippet>\tOK <hex>" or "<snippet>\tERR <first-error>"
BINDIR="$1"
ASL="$BINDIR/asl"
P2BIN="$BINDIR/p2bin"
W=$(mktemp -d /home/volence/sonic_hacks/.isa-cov-work/probe/run.XXXXXX)
while IFS= read -r snip; do
  [ -z "$snip" ] && continue
  rm -f "$W/t.p" "$W/t.bin" "$W/t.log"
  printf '\tcpu 68000\n\torg 0\n\t%s\n' "$snip" > "$W/t.asm"
  AS_MSGPATH="$BINDIR" USEANSI=n "$ASL" -cpu 68000 -q -U -olist /dev/null \
      -o "$W/t.p" "$W/t.asm" > "$W/t.log" 2>&1
  rc=$?
  if [ $rc -ne 0 ] || [ ! -f "$W/t.p" ]; then
    msg=$(grep -a -m1 -E 'error|Error' "$W/t.log" | tr -d '\r' | sed 's/^[[:space:]]*//')
    [ -z "$msg" ] && msg=$(head -c 200 "$W/t.log" | tr '\n' ' ')
    printf '%s\tERR %s\n' "$snip" "$msg"
    continue
  fi
  "$P2BIN" "$W/t.p" "$W/t.bin" >/dev/null 2>&1
  if [ ! -f "$W/t.bin" ]; then printf '%s\tERR p2bin-failed\n' "$snip"; continue; fi
  hex=$(od -An -tx1 -v "$W/t.bin" | tr -s ' ' | tr -d '\n' | sed 's/^ //;s/ $//' | tr 'a-f' 'A-F')
  printf '%s\tOK %s\n' "$snip" "$hex"
done
rm -rf "$W"
