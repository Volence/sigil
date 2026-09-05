	cpu 68000
	padding off
	org 0
m3 macro
	dc.b $A0
	rept 3
	dc.b $C0
	exitm
	dc.b $C1
	endr
	dc.b $A1
	endm
	m3
	dc.b $FF
