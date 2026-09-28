	cpu 68000
	nop
	move.b d0,Fwd(a1,d1.w)
L:	dc.w L
Fwd	equ $40
