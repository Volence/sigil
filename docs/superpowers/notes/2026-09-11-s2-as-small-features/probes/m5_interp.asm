	cpu 68000
	padding off
	org 0
pal	macro path
	message "[path]"
	endm
	pal Special Stage 1 2p.bin
	dc.b $EE
	end
