	cpu 68000
	org $1200
m	macro
	nop
	dc.w $$x
	endm
A1:	nop
$$x:	nop
	m
