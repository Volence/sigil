	cpu 68000
	padding off
	org 0
	move.w ()(a0),d0
	dc.b $EE
	end
