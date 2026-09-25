	cpu 68000
	org $1200
m	macro
$$x:	nop
	dc.w $$x
	endm
A1:	nop
	m
	m
	m
