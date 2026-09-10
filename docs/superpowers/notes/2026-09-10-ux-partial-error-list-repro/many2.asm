	cpu 68000
	move.w	d0,d1
	moveq	#$12,d0
	move.w	#$1234,d1
	bra.s	Nowhere1
	moveq	#$56,d1
	bra.s	Nowhere2
	move.w	d0,d1
