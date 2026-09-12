	cpu 68000
	padding off
	org 0
m	macro pa=1x2
	dc.b "pa"
	endm
	m
	dc.b $EE
	end
