#!/bin/zsh
# Every shape this parcel measured, run against ONE sigil binary.
#
#   matrix.sh                  runs the branch-tip binary named below
#   SIG=<path> matrix.sh       runs any other, which is how the BEFORE column
#                              in the note was produced (a sigil built from
#                              master 742c7366)
#
# Rows tagged CONTROL pin behaviour the parcel must NOT change.
SIG=${SIG:-/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-aaae9c7d6e2503586/.target-land/release/sigil}
WORK=$(mktemp -d)
cd $WORK
trap "rm -rf $WORK" EXIT

run() {
  printf '%s\n' "$1" > m.asm
  echo "--- [$2]"
  $SIG m.asm --hex 2>&1
  echo "   exit=$?"
}
run $'\tcpu 68000\nig function p,$100\n\tdc.l ig(a0)' 'V1 fn ignored arg'
run $'\tcpu 68000\nus function p,p+1\n\tdc.l us(a0)' 'V2 fn used arg'
run $'\tcpu 68000\n\tdc.l a0+1' 'V3 dc.l compound'
run $'\tcpu 68000\n\tdc.l a0' 'V4 dc.l bare'
run $'\tcpu 68000\n\tdc.b a0' 'dc.b bare'
run $'\tcpu 68000\n\tdc.b a0+1' 'dc.b compound'
run $'\tcpu 68000\n\tdc.w a0' 'dc.w bare'
run $'\tcpu 68000\n\tdc.w a0+1' 'dc.w compound'
run $'\tcpu 68000\n\tds.b a0' 'ds.b count'
run $'\tcpu 68000\n\torg a0' 'org'
run $'\tcpu 68000\n\talign a0\n\tdc.l 1' 'align'
run $'\tcpu 68000\n\trept a0\n\tdc.l 1\n\tendr' 'rept count'
run $'\tcpu 68000\n\twhile a0\n\tdc.l 1\n\tendw' 'while cond'
run $'\tcpu 68000\n\tif a0\n\tdc.l 1\n\tendc' 'if cond'
run $'\tcpu 68000\n\tmove.w #a0,d0' 'move.w imm bare'
run $'\tcpu 68000\n\tmove.w #a0+1,d0' 'move.w imm compound'
run $'\tcpu 68000\n\tmove.l #a0,d0' 'move.l imm bare'
run $'\tcpu 68000\n\tmoveq #a0,d0' 'moveq imm'
run $'\tcpu 68000\n\tmove.w a0+1,d0' 'move.w abs ea'
run $'\tcpu 68000\n\tjsr a0' 'jsr bare'
run $'\tcpu 68000\n\tjmp a0+1' 'jmp compound'
run $'\tcpu 68000\n\tlea (a0).l,a1' 'lea abs.l'
run $'\tcpu 68000\nX\tequ a0\n\tdc.l X' 'equ'
run $'\tcpu 68000\n\tdc.l sp' 'sp'
run $'\tcpu 68000\n\tdc.l A0' 'A0 uppercase'
run $'\tcpu 68000\n\tdc.l a0+a1' 'two registers'
run $'\tcpu 68000\n\tdc.l a0+zz' 'register + undefined'
run $'\tcpu 68000\na0\tequ 5\n\tdc.l a0' 'CONTROL: a0 defined'
run $'\tcpu 68000\n\tdc.l zz' 'CONTROL: undefined bare'
run $'\tcpu 68000\n\tdc.l zz+1' 'CONTROL: undefined compound'
run $'\tcpu z80\n\tdw hl' 'CONTROL: z80 dw hl'
run $'\tcpu 68000\n\tmove.w d0,d1\n\tdc.l 1' 'CONTROL: sanity'
