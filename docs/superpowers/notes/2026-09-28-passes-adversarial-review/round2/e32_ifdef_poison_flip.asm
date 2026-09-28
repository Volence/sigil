	cpu 68000
X	equ Fwd
	ifdef X
Fwd	equ 1
	dc.b $A1
	else
Fwd	equ 2
	dc.b $A2
	endif
	dc.b X
