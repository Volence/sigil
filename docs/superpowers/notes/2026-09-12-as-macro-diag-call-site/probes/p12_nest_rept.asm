inner macro
	bogus_inner_rept
	endm
outer macro
	rept 2
	inner
	endm
	endm
	cpu 68000
	outer
