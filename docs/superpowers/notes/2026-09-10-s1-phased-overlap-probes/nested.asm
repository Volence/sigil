	cpu 68000
	padding off
	org 0
	set .rep, 2
	set .val, $21
	rept 4
		rept .rep
			dc.b .val
		endr
	set .val, .val+(+1)
	endr
	end
