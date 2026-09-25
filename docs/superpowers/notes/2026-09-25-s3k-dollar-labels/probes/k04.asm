	cpu 68000
	org $1200
m	macro
	nop
$$x	label	*
	endm
Lab1:	nop
	m
	dc.w $$x
