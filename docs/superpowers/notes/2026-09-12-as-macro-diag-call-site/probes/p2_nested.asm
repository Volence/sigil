inner macro
	bogus_inner
	endm
outer macro
	nop
	inner
	endm
	cpu 68000
	outer
