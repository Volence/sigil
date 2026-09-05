	cpu 68000
	padding off
	org 0
	dc.b $11
mz macro
	dc.b $A0
	exitm
	dc.b $A1
	endm
	rept 2
	dc.b $C0
	mz
	dc.b $C1
	endr
	dc.b $22
