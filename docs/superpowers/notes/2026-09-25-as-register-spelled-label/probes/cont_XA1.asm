	cpu 68000
	org $1200
XA1:	nop
	move.w	#XA1+2,d0
	dc.w	XA1
