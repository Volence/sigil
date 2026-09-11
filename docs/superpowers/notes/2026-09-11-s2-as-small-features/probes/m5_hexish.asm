	cpu 68000
	padding off
	org 0
pal	macro path
	dc.b "path"
	endm
	pal 0FFh 1F 2G
	dc.b $EE
	end
