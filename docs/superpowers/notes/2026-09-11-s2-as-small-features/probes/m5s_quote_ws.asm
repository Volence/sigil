	cpu 68000
	padding off
	org 0
m	macro pa,pb
	dc.b pa
	message "(pb)"
	dc.b ARGCOUNT
	endm
	m " a ",c
	dc.b $EE
	end
