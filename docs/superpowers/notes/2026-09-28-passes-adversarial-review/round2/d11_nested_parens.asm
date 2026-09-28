	cpu 68000
	nop
	move.b d0,((Fa-2)*(Fb+1))(a1)
After:	nop
Fa	equ After-2
Fb	equ 0
