	cpu 68000
	org $1200
Lab:	nop
$$A1:	nop
	move.w	#$$A1+2,d0
	dc.w	$$A1
