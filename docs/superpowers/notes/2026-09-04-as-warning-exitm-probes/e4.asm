	cpu 68000
	padding off
	org 0
m4 macro
	dc.b $A0
	if 0
	exitm
	endif
	dc.b $A1
	endm
	m4
	dc.b $FF
