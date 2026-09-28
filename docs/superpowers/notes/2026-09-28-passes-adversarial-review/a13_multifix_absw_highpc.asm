	cpu 68000
	phase $9000
	move.w Fwd,d0
After:	nop
Fwd	equ After-$1005
