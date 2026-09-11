	cpu 68000
	padding off
	org 0
pal	macro path
	dc.b "path"
	endm
	pal Special Stage 1.bin
	pal Special Stage 1 2p.bin
	end
