	cpu 68000
	padding off
	org 0
m14 macro
	dc.b $A0
	include "e14inc.asm"
	dc.b $A1
	endm
	m14
	dc.b $FF
