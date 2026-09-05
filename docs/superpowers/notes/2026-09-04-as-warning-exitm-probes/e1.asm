	cpu 68000
	padding off
	org 0
m1 macro v
	dc.b $A0
	if v==1
	exitm
	endif
	dc.b $B0
	endm
	dc.b $11
	m1 0
	dc.b $22
	m1 1
	dc.b $33
