	cpu 68000
	ifdef X
	dc.b $A1
	endif
X set 1
	ifdef X
	dc.b $A2
	endif
X set 2
	ifdef Y
	dc.b $A3
	endif
Y = 1
	ifdef Y
	dc.b $A4
	endif
