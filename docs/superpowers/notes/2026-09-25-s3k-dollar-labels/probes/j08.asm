	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	move.w	#($$x-A1),d1
