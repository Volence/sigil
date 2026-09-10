	cpu 68000
	padding off
	org 0
	set .val, $21
	rept 4
		if 1
			dc.b .val
		endif
	set .val, .val+1
	endr
	end
