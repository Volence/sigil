inner macro
	bogus_inner
	endm
outer macro
	rept 1
	nop
	inner
	endm
	endm
	cpu 68000
	outer
