	cpu 68000
	ifdef B
	dc.b $BB
	endif
	ifdef A
B = 1
	endif
A = 1
	dc.b 0
