	cpu	z80
	org	0100h
HL:	equ	01234h
A0:	equ	05678h
	ld	a,(HL)
	ld	a,(hl)
	ld	A,(HL)
	ld	a,(A0)
	ld	B,C
	ld	a,(IX+3)
	ld	a,(SP)
	end
