	cpu 68000
m	macro
	if N>1
Lp:	dc.b 1
	else
Lp:	dc.b 2,2
	endif
	dc.w Lp
	endm
	m
	m
E:	dc.w E
N equ 2
