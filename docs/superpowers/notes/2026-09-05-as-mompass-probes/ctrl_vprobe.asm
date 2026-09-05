	cpu 68000
	padding off
	org 0
V := W
	dc.b V
	dc.b $11
W:	dc.b $EE
	end
