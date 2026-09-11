	cpu 68000
	padding off
	org 0
pal	macro path
	dc.b "path"
	endm
	pal 2.bin
	dc.b $EE
	end
