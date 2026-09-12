	cpu 68000
	padding off
	org 0
m	macro qq,pa=2p.bin
	dc.b qq,"pa"
	endm
	m 5
	dc.b $EE
	end
