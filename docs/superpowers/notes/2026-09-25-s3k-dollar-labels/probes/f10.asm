	cpu 68000
	org $1200
m	macro
	nop
$$q:	nop
	endm
A1:	nop
	m
	dc.w $$q
