	cpu 68000
	padding off
	org 0
	move.w ().w,d0
	dc.b $EE
	end
