	cpu 68000
	nop
	move.b d0,Fwd(a1)
After:
	if After=6
G = 1
	endif
	ifdef G
H = 2
	endif
	dc.b H
Fwd	equ 0
