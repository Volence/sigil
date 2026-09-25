#!/usr/bin/env bash
# mk_s3kprobe.sh: build probes/s3klevsel.asm from VERBATIM skdisasm text: the
# levselstr macro and the LEVELSELECT page block (sonic3k.macros.asm 130-150),
# the make_art_tile function (macros 114), the character-literal lines of the
# level-select plane code (sonic3k.asm 9950-9951, 9975-9995, with the RAM_start
# absolute destinations kept), and LevelSelectText (sonic3k.asm 10552-10569).
set -eu
S=/home/volence/sonic_hacks/.scratch/s3k-codepage
T=$S/trees/sk-pristine
O=$S/probes/s3klevsel.asm
{
  printf '\tcpu 68000\n'
  printf 'tile_mask = $7FF\n'
  printf 'RAM_start = $FFFF0000\n'
  printf 'planeLocH28 function col,line,(line*$50)+(col*2)\n'
  sed -n '114p' "$T/sonic3k.macros.asm"
  sed -n '130,150p' "$T/sonic3k.macros.asm"
  sed -n '9950,9951p' "$T/sonic3k.asm"
  sed -n '9975,9995p' "$T/sonic3k.asm" | grep -v '^\s*\(bsr\|bra\|jsr\|lea\|move.w\s*#\$\|moveq\|move.w\s*(\|dbf\|cmp\|b[a-z][a-z]\.\|tst\|add\|sub\)'
  printf '\tdc.b "*AZaz09:. "\n'
  sed -n '10552,10569p' "$T/sonic3k.asm"
  printf '\tdc.b "*AZaz09:. "\n'
} > "$O"
echo "wrote $O: $(wc -l < "$O") lines"
