	cpu 68000
	org $1200
A1:	nop
B:	nop
	move.w	#B-A1,d0
