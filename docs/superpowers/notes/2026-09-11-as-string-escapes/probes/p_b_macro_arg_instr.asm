	cpu 68000
	padding off
	org 0
m	macro a
	dc.b "<a>"
	endm
	m \x41
	dc.b $EE
	end
