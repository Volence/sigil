	cpu 68000
	padding off
	org 0
m13 macro
	dc.b $A0
	rept 2
	dc.b $D0
	rept 2
	dc.b $C0
	exitm
	dc.b $C1
	endr
	dc.b $D1
	endr
	dc.b $A1
	endm
	m13
	dc.b $FF
