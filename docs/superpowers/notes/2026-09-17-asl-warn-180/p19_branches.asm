	cpu 68000
	padding off
	org 0
	bra.w	oddlbl
	bsr.w	oddlbl
	dbf	d0,oddlbl
	jmp	oddlbl
	jsr	oddlbl
	dc.b	0
oddlbl:
	dc.b	0
