	cpu 68000
	phase $10000
	move.b d0,Fwd(a1)
After:	nop
	dc.w After
Fwd	equ $3E
