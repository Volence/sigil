	cpu 68000
	padding off
	org 0
	irpc c,\x41B
	dc.b "c"
	endr
	dc.b $EE
	end
