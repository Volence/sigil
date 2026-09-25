	cpu 68000
	org $1200
A1:	nop
	nop
$$x:	nop
	dc.w $$x
	dc.w A1.$$x
