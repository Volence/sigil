	cpu 68000
	padding off
	org 0
m	macro pa
	dc.b "pa"
	endm
	m 2p.bin
	dc.b $EE
	end
