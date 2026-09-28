	cpu 68000
	ifdef X
	dc.b $A1
	endif
	dc.b 0
	ifdef Y
X = 1
	endif
Y = 1
