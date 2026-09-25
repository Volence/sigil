	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	shared	A1
	dc.w $$x
