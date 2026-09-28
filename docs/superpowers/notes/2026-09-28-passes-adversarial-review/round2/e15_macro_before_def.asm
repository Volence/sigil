	cpu 68000
m	macro
	ifdef Z
	dc.b 1
	else
	dc.b 2
	endif
	endm
	m
Z = 1
	m
