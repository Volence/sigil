	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	bra.s	$$x
	beq.w	$$y
	nop
$$y:	nop
