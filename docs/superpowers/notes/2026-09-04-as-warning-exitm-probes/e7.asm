	cpu 68000
	padding off
	org 0
m7 macro
	dc.b $A0
w7 set 0
	while w7<3
	dc.b $C0
	exitm
	dc.b $C1
w7 set w7+1
	endm
	dc.b $A1
	endm
	m7
	dc.b $FF
