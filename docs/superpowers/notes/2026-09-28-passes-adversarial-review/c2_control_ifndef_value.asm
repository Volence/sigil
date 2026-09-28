	cpu 68000
	ifndef B
V = 1
	else
V = 2
	endif
	dc.b V
	ifdef A
B = 1
	endif
A = 1
