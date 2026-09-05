	cpu 68000
	padding off
	org 0
m9 macro
	dc.b $A0
lbl9:	exitm
	dc.b $A1
	endm
	m9
	dc.b $FF
	dc.l lbl9
