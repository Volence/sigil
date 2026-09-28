	cpu 68000
	phase $10002
	move.b d0,Fa-2(a1)
After:	nop
Fa	equ After-$10002
