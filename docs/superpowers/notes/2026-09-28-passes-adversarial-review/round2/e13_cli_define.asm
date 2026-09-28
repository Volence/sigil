	cpu 68000
	ifdef Dx
	dc.b $A1
	endif
Dx set 5
	ifdef Dx
	dc.b $A2
	endif
	dc.b Dx
