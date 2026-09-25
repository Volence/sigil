	cpu 68000
	org $1200
A8:	nop
	move.w	#A8+2,d0
	dc.w	A8
