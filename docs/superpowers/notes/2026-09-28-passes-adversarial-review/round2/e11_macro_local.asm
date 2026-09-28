	cpu 68000
m	macro
	ifdef Lp
	dc.b $A1
	endif
Lp:	dc.b 0
	ifdef Lp
	dc.b $A2
	endif
	endm
	m
	m
	ifdef Lp
	dc.b $A3
	endif
