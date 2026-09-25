	cpu 68000
	org $1200
m	macro
	nop
	dc.w $$q
	endm
A1:	nop
$$q:	nop
	m
B1:	nop
	nop
$$q:	nop
	m
