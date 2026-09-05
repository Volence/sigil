	cpu 68000
	padding off
	org 0
	dc.b $11
	rept 3
	dc.b $C0
	exitm
	dc.b $C1
	endr
	dc.b $22
