	cpu 68000
	org $1200
m	macro
	nop
$$v	equ	*+$20
$$w	set	*+$30
	endm
Lab1:	nop
	m
	dc.w $$v,$$w
