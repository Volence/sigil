	cpu 68000
	padding off
	org 0
m8 macro
	dc.b $A0
	irp x,1,2,3
	dc.b $C0
	exitm
	dc.b $C1
	endm
	dc.b $A1
	endm
	m8
	dc.b $FF
