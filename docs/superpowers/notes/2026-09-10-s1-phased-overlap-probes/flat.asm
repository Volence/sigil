	cpu 68000
	padding off
	org 0
	set .val, $21
	dc.b .val
	set .val, .val+(+1)
	dc.b .val
	set .val, .val+(+1)
	dc.b .val
	end
