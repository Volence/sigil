	cpu 68000
	padding off
	org 0
m	macro pa=2p&q
	dc.b "pa"
	endm
	m
	dc.b $EE
	end
