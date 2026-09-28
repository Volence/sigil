	cpu 68000
mv	macro d
	move.b d0,d(a1)
	endm
	nop
	mv Fa-2
After:	nop
Fa	equ After-2
