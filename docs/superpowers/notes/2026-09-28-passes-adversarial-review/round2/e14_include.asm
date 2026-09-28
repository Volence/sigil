	cpu 68000
	ifdef IncName
	dc.b $A1
	endif
	include "e14.inc"
	ifdef IncName
	dc.b $A2
	endif
AfterInc = 1
	dc.b 0
