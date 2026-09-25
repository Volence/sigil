	cpu 68000
	org $1200
A1:	nop
	cpu 68000
.l1:	nop
	padding off
.l2:	nop
	padding on
.l3:	nop
	supmode on
.l4:	nop
	listing off
.l5:	nop
	listing on
.l6:	nop
	save
.l7:	nop
	restore
.l8:	nop
	page 0
.l9:	nop
	enum	E1,E2
.l10:	nop
	nextenum	E3
.l11:	nop
Str	struct
f1	ds.w	1
Str	endstruct
.l12:	nop
	cpu z80
.l13:	nop
	cpu 68000
.l14:	nop
	charset $41,$11
.l15:	nop
	phase $2000
.l16:	nop
	dephase
.l17:	nop
Q	equ	5
.l18:	nop
	org $1300
.l19:	nop
	align 2
.l21:	nop
	if 1
.l22:	nop
	endif
	rept 1
.l23:	nop
	endm
	ds.b 2
.l24:	nop
Mm	macro
	endm
.l25:	nop
	codepage X
.l26:	nop
