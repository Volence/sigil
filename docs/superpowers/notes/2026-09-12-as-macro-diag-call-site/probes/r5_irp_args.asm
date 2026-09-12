	cpu 68000
mymac macro a,b
	irp x,a,b
	bogus_m x
	endm
	endm
	mymac p1,p2
