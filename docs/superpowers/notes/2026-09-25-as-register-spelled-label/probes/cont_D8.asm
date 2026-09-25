	cpu 68000
	org $1200
D8:	nop
	move.w	#D8+2,d0
	dc.w	D8
