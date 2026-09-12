	cpu 68000
mymac macro arg
	dc.w arg
	endm
	nop
	mymac undefined_arg
