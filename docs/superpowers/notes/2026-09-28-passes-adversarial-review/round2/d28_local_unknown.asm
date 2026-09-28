	cpu 68000
S:
	nop
	move.b d0,.fwd-2(a1)
After:	nop
.fwd	equ After-2
