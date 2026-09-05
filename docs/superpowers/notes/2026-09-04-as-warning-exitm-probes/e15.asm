	cpu 68000
	padding off
	org 0
m15 macro
	dc.b $A0
	rept 3
	dc.b $C0
	m15b
	dc.b $C1
	endr
	dc.b $A1
	endm
m15b macro
	dc.b $B0
	exitm
	dc.b $B1
	endm
	m15
	dc.b $FF
