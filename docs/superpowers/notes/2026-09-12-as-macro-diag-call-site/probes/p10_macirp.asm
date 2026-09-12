	cpu 68000
mymac macro
	irp x,1,2
	bogus_irp x
	endm
	endm
	nop
	mymac
