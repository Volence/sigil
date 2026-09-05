	cpu 68000
	padding off
	org 0
m16 macro n
	dc.b $A0
	switch n
	case 1
	exitm
	case 2
	dc.b $C2
	endcase
	dc.b $A1
	endm
	m16 1
	m16 2
	dc.b $FF
