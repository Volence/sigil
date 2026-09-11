	cpu 68000
	padding off
	org 0
	move.w 0(a0),d0
	dc.b $EE
	end
