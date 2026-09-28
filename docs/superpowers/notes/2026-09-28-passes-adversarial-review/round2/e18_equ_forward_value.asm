	cpu 68000
X	equ Fwd
	ifdef X
	dc.b $A1
	endif
	dc.b 0
Fwd	equ 1
