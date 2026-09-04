	cpu	68000
	org	0
m	macro	px,py
	dc.b	px,py
	endm
	m	1,2,px=9
	end
