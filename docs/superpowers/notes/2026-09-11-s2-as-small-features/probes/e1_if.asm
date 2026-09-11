	cpu 68000
	padding off
	org 0
	if ()
	dc.b 1
	else
	dc.b 2
	endif
	dc.b $EE
	end
