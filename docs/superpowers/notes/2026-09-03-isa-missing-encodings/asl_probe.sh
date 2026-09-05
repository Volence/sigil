#!/bin/bash
# asl_probe.sh [asl-binary-dir] ; reads snippets one per line on stdin,
# prints "<snippet>\tOK <hex>" or "<snippet>\tERR <first-error>"
#
# THE ASSEMBLER IS IDENTIFIED BY DIGEST, NOT BY BANNER — see
# `../asl-reference/`. Four `asl` binaries in this workspace print
# `Macro Assembler 1.42 Beta [Bld 212]` verbatim and are not the same program,
# and one of them answers any operand it declined to value from uninitialized
# memory. The predecessor took the directory as a required bare path and printed
# nothing about it, so a matrix said only that "an asl" produced it.
#
# The argument SURVIVES, because this probe exists to fill `matrix.s1.tsv` and
# `matrix.s2.tsv` — one per shipped build — and a digest pin would delete that.
# With no argument it uses the reference build. Either way the md5 of the binary
# that ran is announced ON STDERR, so it never lands in the TSV, and no matrix is
# minted anonymously.
if [ $# -ge 1 ]; then
    BINDIR="$1"
    [ -x "$BINDIR/asl" ] || { echo "FATAL: no executable asl at $BINDIR/asl" >&2; exit 2; }
    echo "# CROSS-CHECK BUILD: $BINDIR/asl md5 $(md5sum "$BINDIR/asl" | cut -d' ' -f1)" >&2
else
    HERE="$(cd "$(dirname "$0")" && pwd)"
    . "$HERE/../asl-reference/asl_ref.sh" || exit $?
    BINDIR="$ASLDIR"
    echo "# reference build: $BINDIR/asl md5 $(md5sum "$BINDIR/asl" | cut -d' ' -f1)" >&2
fi
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
