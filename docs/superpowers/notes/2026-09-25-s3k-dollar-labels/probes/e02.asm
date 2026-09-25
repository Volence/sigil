	cpu 68000
	org $1200
m	macro
$$l:	nop
	dc.w $$l
	endm
A1:	nop
$$x:	nop
	m
	nop
	m
	dc.w $$x
