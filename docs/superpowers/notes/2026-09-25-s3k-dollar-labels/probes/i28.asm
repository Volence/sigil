	cpu 68000
	org $1200
m	macro
Lb:	nop
	endm
A1:	nop
$$x:	nop
	m
A1b	set	1
$$x:	nop
	dc.w $$x
