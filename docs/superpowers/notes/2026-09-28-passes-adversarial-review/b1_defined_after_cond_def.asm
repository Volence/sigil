	cpu 68000
	ifdef A
B = 1
	endif
	if DEFINED(B)
	dc.b $BB
	endif
A = 1
	dc.b 0
