	cpu 68000
X	equ L
	ifdef X
	dc.w X
	endif
L:	dc.w L
