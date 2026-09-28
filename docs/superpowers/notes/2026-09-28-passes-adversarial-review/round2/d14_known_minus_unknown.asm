	cpu 68000
K	equ 2
	nop
	move.b d0,K-Fa(a1)
After:	nop
Fa	equ After-2
