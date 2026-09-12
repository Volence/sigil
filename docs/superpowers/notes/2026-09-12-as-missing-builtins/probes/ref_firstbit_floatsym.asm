	cpu 68000
	padding off
	org 0
F set 2.5
	dc.l firstbit(F)
	dc.b $EE
	end
