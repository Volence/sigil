	cpu 68000
	ifdef Rg
	dc.b $A1
	endif
Rg	reg d0-d3
	ifdef Rg
	dc.b $A2
	endif
	dc.b 0
