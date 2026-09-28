	cpu 68000
	ifdef C
B = 1
	endif
	ifdef B
A2 = 1
	endif
	ifdef A2
	dc.b $A2
	endif
C = 1
	dc.b 0
