	cpu 68000
	org $1200
m	macro
Lb:	nop
$$q:	nop
	dc.w $$q
	endm
A1:	nop
	m
	nop
	m
