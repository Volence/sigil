	cpu 68000
	org $1200
A1:	nop
$$x:	nop
m3	macro
	nop
	endm
	dc.w $$x
