	cpu 68000
	save
X = 1
	restore
	ifdef X
	dc.b $A1
	endif
	dc.b 0
