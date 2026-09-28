	cpu 68000
	nop
	move.w Fwd,d0
After:	nop
Fwd	equ After+$7FFF-6
