	cpu 68000
	padding off
	org 0
pal	macro path
	dc.b "path"
	endm
	pal a  1   2p
	dc.b $EE
	end
