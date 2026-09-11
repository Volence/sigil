	cpu 68000
	padding off
	org 0
lbl	macro n
Lab_n:	dc.b 1
	endm
	lbl 5
	dc.l Lab_5
	dc.b $EE
	end
