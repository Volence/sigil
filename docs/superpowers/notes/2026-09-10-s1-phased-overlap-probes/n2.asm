	cpu 68000
	padding off
	org 0
	set .val, $21
	rept 4
	set .val, .val+1
		rept 2
			dc.b .val
		endr
	endr
	end
