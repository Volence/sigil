	cpu 68000
	padding off
	org 0
m	macro pa=2p.bin,qq
	dc.b "pa",qq
	endm
	m ,5
	dc.b $EE
	end
