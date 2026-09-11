	cpu 68000
	padding off
	org 0
m macro pa,pb
	dc.b pb
	endm
	m 5
	dc.b $EE
	end
