#!/usr/bin/env python3
"""mkprobes.py: write every probe source into probes/. Accepted grids are one file
each (every line must assemble); each refusal is its own file (one refused line
after `cpu 68000`), so a refusal never hides another line's value."""
import os

P = "/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex/probes"
os.makedirs(P, exist_ok=True)
HDR = "\tcpu 68000\n"


def w(name, lines):
    with open(os.path.join(P, name + ".asm"), "w") as f:
        f.write(HDR + "".join((l if ":" in l.split(" ")[0] else "\t" + l) + "\n" for l in lines))


# ---- BCD / extended family, accepted grids ----------------------------------
w("x01", """addx.b d3,d5
addx.w d3,d5
addx.l d3,d5
addx d3,d5
addx.b -(a3),-(a5)
addx.w -(a3),-(a5)
addx.l -(a3),-(a5)
addx -(a3),-(a5)
addx.w d6,d1
addx.l -(sp),-(a6)
addx.w -(a1),-(a7)
subx.b d3,d5
subx.w d3,d5
subx.l d3,d5
subx d3,d5
subx.b -(a3),-(a5)
subx.w -(a3),-(a5)
subx.l -(a3),-(a5)
subx -(a3),-(a5)
subx.w d6,d1
subx.l -(sp),-(a6)
ADDX.W D3,D5
SUBX.L -(A3),-(A5)""".split("\n"))

w("x02", """abcd d3,d5
abcd.b d3,d5
abcd -(a3),-(a5)
abcd.b -(a3),-(a5)
abcd d6,d1
abcd -(a1),-(a2)
abcd -(sp),-(a6)
sbcd d3,d5
sbcd.b d3,d5
sbcd -(a3),-(a5)
sbcd.b -(a3),-(a5)
sbcd d6,d1
sbcd -(a1),-(sp)
ABCD -(A3),-(A5)""".split("\n"))

w("x03", """negx.b d3
negx.w d3
negx.l d3
negx d5
negx.b (a3)
negx.w (a3)+
negx.l -(a3)
negx.w $1234(a5)
negx.b -$12(a5,d6.w)
negx.l $12(a5,a2.l)
negx.w ($1234).w
negx.l ($123456).l
nbcd d3
nbcd.b d5
nbcd (a3)
nbcd (a3)+
nbcd -(a3)
nbcd $1234(a5)
nbcd -$12(a5,d6.w)
nbcd $12(a5,a2.l)
nbcd ($1234).w
nbcd ($123456).l""".split("\n"))

# ---- BCD / extended family, refusals (one per file) -------------------------
XR = [
    "abcd.w d3,d5", "abcd.l d3,d5", "abcd d3,-(a5)", "abcd -(a3),d5", "abcd (a3),(a5)",
    "abcd (a3)+,(a5)+", "addx d3,-(a5)", "addx -(a3),d5", "addx a3,a5", "addx #1,d5",
    "addx (a3),(a5)", "subx d3,-(a5)", "subx.w (a3)+,(a5)+", "sbcd.w d3,d5", "negx a3",
    "negx #1", "negx 4(pc)", "negx.w d3,d5", "nbcd.w d3", "nbcd a3", "nbcd #1",
    "nbcd.l (a3)", "addx.s d3,d5", "abcd d3", "addx d3", "negx.w", "nbcd *(pc,d3.w)",
    "subx.b a3,d5", "sbcd d3,a5",
]
for i, l in enumerate(XR, 1):
    w("xr%02d" % i, [l])

# ---- (d8,PC,Xn): every source-EA instruction that admits it ------------------
# Each line's target is a label T placed so the displacements are non-symmetric
# and non-zero; index registers are distinct from the other operand's.
pcx_lines = """move.b T(pc,d3.w),d5
move.w T(pc,d3.l),d5
move.l T(pc,a3.w),d5
move.w T(pc,a3.l),(a5)
move.l T(pc,d6.w),$1234(a5)
movea.w T(pc,d3.w),a5
movea.l T(pc,a2.l),a5
move.w T(pc,d3.w),a5
add.b T(pc,d3.w),d5
add.w T(pc,d3.l),d5
add.l T(pc,a3.w),d5
adda.w T(pc,d3.w),a5
adda.l T(pc,d3.l),a5
sub.w T(pc,d3.w),d5
suba.l T(pc,a4.w),a5
and.w T(pc,d3.w),d5
or.l T(pc,d3.l),d5
cmp.b T(pc,d3.w),d5
cmp.w T(pc,a3.l),d5
cmpa.w T(pc,d3.w),a5
cmpa.l T(pc,d3.w),a5
muls.w T(pc,d3.w),d5
mulu.w T(pc,d3.l),d5
divs.w T(pc,a3.w),d5
divu.w T(pc,d3.w),d5
btst d5,T(pc,d3.w)
btst #3,T(pc,d3.l)
T: dc.w 1
lea T(pc,d3.w),a5
lea T(pc,a3.l),a5
pea T(pc,d3.w)
jmp T(pc,d3.w)
jsr T(pc,a3.l)
movem.w T(pc,d0.w),d2-d3
movem.l T(pc,d3.l),d0-d2/a4
movem.w T(pc,a3.w),a0/d6
movem.l T(pc,a2.l),d0-d7/a0-a6
move.w T(pc,d3.w),ccr
move.w T(pc,d3.w),sr
move T(pc,d3.w),ccr
move.w T(pc,d3),d5
move.w T(PC,D3.W),d5
move.w T(Pc,d3.W),d5
move.w (T,pc,d3.w),d5
move.w (T,PC,a3.L),d5
move.w T(pc,sp.w),d5
move.w T(pc,a7.l),d5
movem.w (T,pc,d3.w),d2/d4
movem.w T(pc,d3),d2/d4""".split("\n")
w("pcx01", pcx_lines)

# Backward target, and the S3K shape verbatim.
w("pcx02", """B: dc.w $1111,$2222
move.w B(pc,d3.w),d5
movem.w B(pc,d0.w),d2-d3
lea B(pc,a3.l),a5
jmp B(pc,d6.w)
btst #7,B(pc,d3.w)""".split("\n"))

w("pcx03", """movem.w word_82872(pc,d0.w),d2-d3
movem.w word_82832(pc,d0.w),d4-d5
word_82832: dc.w 1,2,3,4
word_82872: dc.w 5,6,7,8""".split("\n"))

# Displacement boundaries. For every form here the extension word follows the
# opcode word, so the PC base is the instruction's own address + 2; btst #n puts
# its bit-number word first and its base is address + 4, as does movem with its
# register-mask word.
w("pcb01", """move.w *+2+127(pc,d3.w),d5
move.w *+2-128(pc,d3.w),d5
movem.w *+4+127(pc,d3.w),d2-d3
movem.w *+4-128(pc,d3.w),d2-d3
lea *+2+127(pc,a3.l),a5
lea *+2-128(pc,a3.l),a5
btst #3,*+4+127(pc,d3.w)
btst #3,*+4-128(pc,d3.w)
btst d5,*+2+127(pc,d3.w)
jmp *+2-1(pc,d3.w)""".split("\n"))
PCB = [
    "move.w *+2+128(pc,d3.w),d5", "move.w *+2-129(pc,d3.w),d5",
    "movem.w *+4+128(pc,d3.w),d2-d3", "movem.w *+4-129(pc,d3.w),d2-d3",
    "btst #3,*+4+128(pc,d3.w)", "btst #3,*+4-129(pc,d3.w)",
    "lea *+2+128(pc,a3.l),a5", "lea *+2-129(pc,a3.l),a5",
]
for i, l in enumerate(PCB, 1):
    w("pcbr%02d" % i, [l])

# Destinations and forms whose EA class excludes (d8,PC,Xn); index-register refusals.
PCR = [
    "move.w d5,T(pc,d3.w)\nT: dc.w 1",
    "clr.w T(pc,d3.w)\nT: dc.w 1",
    "tst.w T(pc,d3.w)\nT: dc.w 1",
    "addi.w #1,T(pc,d3.w)\nT: dc.w 1",
    "cmpi.w #1,T(pc,d3.w)\nT: dc.w 1",
    "add.w d5,T(pc,d3.w)\nT: dc.w 1",
    "movem.w d2-d3,T(pc,d0.w)\nT: dc.w 1",
    "bset #1,T(pc,d3.w)\nT: dc.w 1",
    "neg.w T(pc,d3.w)\nT: dc.w 1",
    "move.w T(pc,d3.b),d5\nT: dc.w 1",
    "move.w T(pc,d8.w),d5\nT: dc.w 1",
    "move.w T(pc,pc.w),d5\nT: dc.w 1",
    "move.w T(pc,d3.w*2),d5\nT: dc.w 1",
    "movem.w T(pc,d3.b),d2-d3\nT: dc.w 1",
    "movem.w T(pc,x.w),d2-d3\nT: dc.w 1",
    "nbcd T(pc,d3.w)\nT: dc.w 1",
    "negx.w T(pc,d3.w)\nT: dc.w 1",
    "move.w T(pc,d3.w),usp\nT: dc.w 1",
    "lsl.w T(pc,d3.w)\nT: dc.w 1",
    "scc T(pc,d3.w)\nT: dc.w 1",
    "move.w T(pc),T(pc)\nT: dc.w 1",
    "move.w T(pc,d3.w),T(pc,d4.w)\nT: dc.w 1",
    "movem.l d0-d2,T(pc)\nT: dc.w 1",
]
for i, l in enumerate(PCR, 1):
    w("pcr%02d" % i, l.split("\n"))

# movem through (d16,PC): the same memory-EA path as the indexed form.
w("pcd01", """movem.w T(pc),d2-d3
movem.l (T,pc),d0/a1
T: dc.w 1
movem.w T(pc),a2/d6""".split("\n"))

print("written", len(os.listdir(P)))
