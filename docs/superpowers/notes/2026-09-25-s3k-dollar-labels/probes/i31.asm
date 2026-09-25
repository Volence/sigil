	cpu 68000
	org $1200
A1:	nop
	nop
$$x:	nop
	cmpi.w	#$$x,d0
	dc.w $$x,$1234
