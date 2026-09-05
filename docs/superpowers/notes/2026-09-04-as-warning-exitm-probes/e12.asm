	cpu 68000
	padding off
	org 0
m12 macro
	dc.b $A0
lbl12:	dc.b $A1
	endm
	m12
	dc.b $FF
	dc.l lbl12
