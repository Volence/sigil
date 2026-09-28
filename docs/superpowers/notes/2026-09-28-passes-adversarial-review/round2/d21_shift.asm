	cpu 68000
	nop
	move.b d0,Fwd>>16(a1)
After:	dc.w After
Fwd	equ $123456
