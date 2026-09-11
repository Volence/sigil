	cpu 68000
	padding off
	org 0
	dc.b lastbit(lastbit($FFFEB))
	dc.b $EE
	end
