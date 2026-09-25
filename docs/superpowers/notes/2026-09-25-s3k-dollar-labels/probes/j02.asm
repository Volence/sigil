	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	move.w	#$$x,d0
	lea	$$x(pc),a0
	move.w	($$x).w,d2
	dc.l	$$x+$10000
