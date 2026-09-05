	cpu 68000
	padding off
	org 0
m10 macro
	dc.b $A0
	exitm 1,2,junk
	dc.b $A1
	endm
	m10
	dc.b $FF
