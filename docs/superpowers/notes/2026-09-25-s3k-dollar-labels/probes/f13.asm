	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	if 1
	nop
	endif
	dc.w $$x
