	cpu 68000
	org $1200
Lab1:	nop
$$x:	nop
	move.w	#$$x-Lab1,d1
	move.w	#($$x-Lab1)*3,d2
