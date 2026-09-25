	cpu 68000
	org $1200
A10:	nop
	move.w	#A10+2,d0
	dc.w	A10
