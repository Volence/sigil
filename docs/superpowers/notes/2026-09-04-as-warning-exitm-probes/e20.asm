	cpu 68000
	padding off
	phase 0
m20 macro sel
	dc.b $A0
	switch sel
	case "stop"
	exitm
	case "go"
	dc.b $C2
	endcase
	dc.b $A1
	endm
	m20 "stop"
	m20 "go"
	dc.b $FF
