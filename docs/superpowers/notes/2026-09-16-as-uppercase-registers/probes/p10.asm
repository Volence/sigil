	cpu	z80
HL:	equ	01234h
A0:	equ	05678h
	ld	a,(HL)
	ld	A,(HL)
	ld	B,C
	ld	a,(IX+3)
	ld	a,(A0)
	ld	BC,01234h
	jr	NZ,$
	ex	(SP),HL
	end
