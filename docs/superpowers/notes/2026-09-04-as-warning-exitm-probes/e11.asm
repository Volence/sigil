	cpu 68000
	padding off
	org 0
	dc.b $11
	irp x,1,2,3
	dc.b $C0
	exitm
	dc.b $C1
	endm
	dc.b $22
