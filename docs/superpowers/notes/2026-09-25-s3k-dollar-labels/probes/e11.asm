	cpu 68000
	org $1200
m2	macro arg
	nop
	dc.w arg
	endm
A1:	nop
$$x:	nop
	m2 $$x
