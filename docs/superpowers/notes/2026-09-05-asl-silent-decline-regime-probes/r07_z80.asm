; The same shape on the other CPU this workspace assembles.
;
; Z80 syntax, not 68000: `$` is the location counter here, so hex is written
; `0A101h`, and there is no `padding` directive.
	cpu z80
fu	function p,(p*7)+100h
	org 1000h
	ld	bc,0A101h
	ld	bc,fu(hl)
	ld	bc,0A202h
	ld	bc,fu(a)
	ld	bc,0A303h
	ld	bc,fu(bc)
	ld	bc,0A404h
	ld	bc,fu(ix)
	ld	bc,0A505h
	dw	fu(hl)
	ld	bc,0A606h
	dw	hl
	ld	bc,0A707h
	ld	bc,fu(5)	; control: must be 123h
	ld	bc,0A808h
