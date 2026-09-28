	cpu 68000
	ifdef A
mm	macro
	dc.b $AA
	endm
	endif
	mm
A = 1
