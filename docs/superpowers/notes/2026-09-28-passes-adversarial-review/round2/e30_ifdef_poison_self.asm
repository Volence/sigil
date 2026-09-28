	cpu 68000
X	equ Fwd
	ifdef X
Fwd	equ 1
	dc.b $A1
	endif
	dc.b 0
