	cpu 68000
	org $1200
m	macro
	nop
	dc.w $$q
	endm
A1:	nop
	m
$$q:	nop
