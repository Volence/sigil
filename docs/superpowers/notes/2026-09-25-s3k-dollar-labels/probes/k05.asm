	cpu 68000
	org $1200
Lab1:	nop
$$x:	nop
	pushv	Lab1
	dc.w $$x
