	cpu 68000
mymac macro
cnt set 0
	while cnt<2
	bogus_in_while
cnt set cnt+1
	endm
	endm
	nop
	mymac
