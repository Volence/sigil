	cpu 68000
	padding off
	org 0
	if MOMPASS=2
	dc.b $AA
	endif
	dc.b $11
	dc.w Later-*
Later:
	end
