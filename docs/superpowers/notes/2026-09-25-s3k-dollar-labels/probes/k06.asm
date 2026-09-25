	cpu 68000
	org $1200
m	macro
	dc.w $$q
Lb:	nop
$$q:	nop
	endm
Lab1:	nop
	m
