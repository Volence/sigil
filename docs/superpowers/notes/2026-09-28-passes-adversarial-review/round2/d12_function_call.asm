	cpu 68000
ff	function x,x-2
	nop
	move.b d0,ff(Fa)(a1)
After:	nop
Fa	equ After-2
