	cpu 68000
m	macro
	bra.w .lp
	if N=1
	dc.w $1111
	endif
.lp:	nop
	dc.w .lp
	endm
A:	m
B:	m
N equ 1
