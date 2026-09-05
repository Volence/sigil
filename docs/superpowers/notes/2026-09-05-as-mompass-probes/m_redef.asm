	cpu 68000
	padding off
	org 0
MOMPASS = 7
	dc.b MOMPASS
	dc.b $11
	dc.w Later-*
Later:
	end
