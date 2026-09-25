	cpu 68000
	org $1200
A1_2:	nop
	move.w	#A1_2+2,d0
	dc.w	A1_2
