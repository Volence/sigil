	cpu 68000
	org $1200
A1:	nop
$$x:	nop
$$v	equ	$$x+$21
	dc.w $$v,$$x*2
B1:	nop
$$x:	nop
$$v	equ	$$x+$43
	dc.w $$v
