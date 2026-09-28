	cpu 68000
	ifdef St
	dc.b $A1
	endif
St	equ "ab"
	ifdef St
	dc.b $A2
	endif
	dc.b 0
