	cpu 68000
	org $1200
m	macro
Lb:	nop
	endm
A1:	nop
$$x:	nop
	m
	dc.w $$x
