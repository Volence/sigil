	cpu 68000
	org $1200
A1:	nop
$$x:	nop
B	equ	$$x
	dc.w B
