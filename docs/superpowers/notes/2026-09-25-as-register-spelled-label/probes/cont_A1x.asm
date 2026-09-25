	cpu 68000
	org $1200
A1x:	nop
	move.w	#A1x+2,d0
	dc.w	A1x
