	cpu 68000
	ifdef B
	dc.b $BB
	endif
B = 1
	dc.b 0
