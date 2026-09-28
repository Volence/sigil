	cpu 68000
	nop
	move.b d0,Fa+Fb-Fc*2(a1)
After:	nop
Fa	equ After-4
Fb	equ 0
Fc	equ 0
