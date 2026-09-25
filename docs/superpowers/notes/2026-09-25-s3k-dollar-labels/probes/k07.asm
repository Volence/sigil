	cpu 68000
	org $1200
m	macro
	dc.w $$q
Lb:	nop
	nop
$$q:	nop
	dc.w $$q
	endm
Lab1:	nop
	nop
$$q:	nop
	m
	m
