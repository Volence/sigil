	cpu 68000
	org $1200
m	macro
Lb:	nop
	endm
A1:	nop
	m
$$z:	nop
	dc.w $$z
	m
$$z:	nop
	dc.w $$z
