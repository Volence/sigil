	cpu 68000
	padding off
	org 0
lbl	macro n
Lab_n:	dc.b 1
	endm
	lbl 2p
	dc.l Lab_2p
	dc.b $EE
	end
