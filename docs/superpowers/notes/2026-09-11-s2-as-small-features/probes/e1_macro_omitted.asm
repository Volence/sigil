	cpu 68000
	padding off
	org 0
m macro a,b
	dc.b (a)|(b)
	endm
	m 5
	dc.b $EE
	end
