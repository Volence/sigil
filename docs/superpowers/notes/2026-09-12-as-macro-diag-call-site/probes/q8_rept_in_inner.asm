inner macro
	rept 2
	bogus_inner_rept
	endm
	endm
outer macro
	nop
	inner
	endm
	cpu 68000
	outer
