	cpu 68000
m	macro
	dc.w Lp
	if N=1
	dc.w $1111
	endif
Lp:	nop
	endm
	m
	m
N equ 1
