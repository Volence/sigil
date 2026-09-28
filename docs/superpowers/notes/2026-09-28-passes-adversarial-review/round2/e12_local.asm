	cpu 68000
S1:
	ifdef .x
	dc.b $A1
	endif
.x:	dc.b 0
	ifdef .x
	dc.b $A2
	endif
S2:
	ifdef .x
	dc.b $A3
	endif
