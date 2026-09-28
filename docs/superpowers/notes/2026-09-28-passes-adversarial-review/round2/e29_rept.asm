	cpu 68000
	rept 2
	ifdef R
	dc.b $A1
	endif
R set 1
	endm
	dc.b 0
