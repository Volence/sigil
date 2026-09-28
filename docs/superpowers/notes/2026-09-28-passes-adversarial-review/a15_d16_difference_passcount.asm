	cpu 68000
	nop
	move.b d0,Fld-Base(a1)
After:	nop
	dc.w After
Base	equ 0
Fld	equ $40
