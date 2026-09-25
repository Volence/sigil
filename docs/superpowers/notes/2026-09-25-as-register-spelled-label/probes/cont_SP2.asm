	cpu 68000
	org $1200
SP2:	nop
	move.w	#SP2+2,d0
	dc.w	SP2
