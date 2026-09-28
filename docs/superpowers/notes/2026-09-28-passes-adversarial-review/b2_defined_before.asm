	cpu 68000
	if DEFINED(B)
	dc.b $BB
	else
	dc.b $CC
	endif
	ifdef A
B = 1
	endif
A = 1
