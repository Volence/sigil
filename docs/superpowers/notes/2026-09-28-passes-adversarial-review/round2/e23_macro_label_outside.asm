	cpu 68000
m	macro
Lp:	dc.b 0
	endm
	m
	ifdef Lp
	dc.b $A3
	endif
	dc.w Lp
