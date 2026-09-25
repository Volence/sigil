	cpu 68000
	org $1200
M	macro
A1:	nop
	move.w	#A1+2,d0
	endm
	M
