	cpu 68000
	org $1200
Lab1:	nop
$$x:	nop
	dc.w	$$x-Lab1+$10
