	cpu 68000
	org $1200
	phase $2400
A1:	nop
$$x:	nop
	dephase
	dc.w $$x
