	cpu 68000
	org $1200
A1:	nop
	rept 2
	nop
$$r:	nop
	dc.w $$r
	endm
