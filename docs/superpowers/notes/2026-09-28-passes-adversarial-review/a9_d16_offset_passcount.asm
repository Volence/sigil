	cpu 68000
	nop
	move.b d0,Fwd-2(a1)
After:	nop
	dc.w After
Fwd	equ $40
