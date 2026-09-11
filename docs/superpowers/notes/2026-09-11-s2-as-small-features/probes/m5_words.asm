	cpu 68000
	padding off
	org 0
pal	macro path
	dc.b "path"
	endm
	pal 1st 2nd 3rd 4th.bin
	dc.b $EE
	end
