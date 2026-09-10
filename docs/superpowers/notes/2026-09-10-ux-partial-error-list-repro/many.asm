	cpu 68000
	mvoe	d0,d1
	moveq	#$1234,d0
	badop	#$1234,d1
	bra.s	Nowhere1
	moveq	#$5678,d1
	bra.s	Nowhere2
	zzz	d0
