	cpu 68000
	org $1200
A1:	nop
.l:	nop
	move.w	#A1.l+2,d0
	dc.w	A1.l
