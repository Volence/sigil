inner macro
	nop
	bogus_inner
	endm
mid macro
	nop
	nop
	inner
	endm
outer macro
	mid
	endm
	cpu 68000
	outer
