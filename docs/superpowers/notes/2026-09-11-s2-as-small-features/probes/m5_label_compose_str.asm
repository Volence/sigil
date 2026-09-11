	cpu 68000
	padding off
	org 0
lbl	macro n
	dc.b "Lab_n"
	endm
	lbl 2p
	dc.b $EE
	end
