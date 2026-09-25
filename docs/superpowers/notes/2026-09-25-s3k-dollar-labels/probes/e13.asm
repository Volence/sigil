	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	rept 2
	nop
	endm
	dc.w $$x
